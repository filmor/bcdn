defmodule Bcdn.Repo.Migrations.CreateFeeds do
  use Ecto.Migration

  def change do
    create table(:feeds) do
      add :name, :string
      add :url, :string

      timestamps()
    end
  end
end
