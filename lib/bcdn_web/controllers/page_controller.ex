defmodule BcdnWeb.PageController do
  use BcdnWeb, :controller

  def index(conn, _params) do
    render(conn, "index.html")
  end
end
