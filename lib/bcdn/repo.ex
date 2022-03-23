defmodule Bcdn.Repo do
  use Ecto.Repo,
    otp_app: :bcdn,
    adapter: Ecto.Adapters.SQLite3
end
