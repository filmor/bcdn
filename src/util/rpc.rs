use tokio::sync::{mpsc, oneshot};
use futures::FutureExt;

type Msg<Question, Answer> = (Question, oneshot::Sender<Answer>);

const BUF_SIZE: usize = 4;

pub fn rpc<Question, Answer>() -> (RpcHandle<Question, Answer>, RpcReceiver<Question, Answer>) {
    let (tx, rx) = mpsc::channel(BUF_SIZE);
    (RpcHandle { tx }, RpcReceiver { rx })
}

pub struct RpcReceiver<Question, Answer> {
    rx: mpsc::Receiver<Msg<Question, Answer>>,
}

unsafe impl<Question, Answer> Send for RpcReceiver<Question, Answer> {}

impl<Question, Answer> Unpin for RpcReceiver<Question, Answer> {}

#[derive(Clone)]
pub struct RpcHandle<Question, Answer> {
    tx: mpsc::Sender<Msg<Question, Answer>>,
}

impl<Question, Answer> RpcHandle<Question, Answer> {
    pub async fn call(&self, q: Question) -> Result<Answer, RpcError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .clone()
            .send((q, tx))
            .await
            .map_err(|_| RpcError::ReceiverClosed)?;
        rx.await.map_err(|_| RpcError::ReceiverClosed)
    }
}

impl<Question, Answer> RpcReceiver<Question, Answer> {
    pub fn try_reply<F>(&mut self, func: F) -> Result<(), RpcError>
    where
        F: FnOnce(Question) -> Answer,
    {
        let (q, tx) = self.rx.recv()
            .now_or_never()
            .ok_or(RpcError::Empty)?
            .ok_or(RpcError::ReceiverClosed)?
            ;

        tx.send(func(q)).map_err(|_| RpcError::SenderClosed)
    }

    pub async fn reply<F>(&mut self, func: F) -> Result<(), RpcError>
    where
        F: FnOnce(Question) -> Answer,
    {
        let (q, tx) = self.rx.recv().await.ok_or(RpcError::SenderClosed)?;
        tx.send(func(q)).map_err(|_| RpcError::SenderClosed)
    }
}

use thiserror::Error;

#[derive(Debug, Clone, Copy, Error)]
pub enum RpcError {
    #[error("Receiver end closed")]
    ReceiverClosed,
    #[error("Sender end closed")]
    SenderClosed,
    #[error("No pending queries")]
    Empty,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn basic_test() {
        #[derive(Clone)]
        enum Q {
            Ping(u32),
            Quit,
        }

        #[derive(PartialEq, Debug, Clone)]
        enum A {
            Pong(u32),
            Done,
        }

        let (tx, mut rx) = rpc();

        let responder = tokio::spawn(async move {
            let mut cont = true;
            while cont {
                rx.reply(|q| match q {
                    Q::Ping(value) => A::Pong(value),
                    Q::Quit => {
                        cont = false;
                        A::Done
                    }
                })
                .await.unwrap();
            }
        });

        assert_eq!(tx.call(Q::Ping(1)).await.unwrap(), A::Pong(1));

        let tasks: Vec<_> = (0..1024 * 100)
            .map(|i| {
                let tx = tx.clone();
                tokio::spawn(async move {
                    let fut = tx.call(Q::Ping(i));
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    assert_eq!(fut.await.unwrap(), A::Pong(i));
                })
            })
            .collect();

        for handle in tasks.into_iter() {
            handle.await.unwrap();
        }

        assert_eq!(tx.call(Q::Quit).await.unwrap(), A::Done);

        responder.await.unwrap();
    }
}
