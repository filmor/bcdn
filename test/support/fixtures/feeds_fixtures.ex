defmodule Bcdn.FeedsFixtures do
  @moduledoc """
  This module defines test helpers for creating
  entities via the `Bcdn.Feeds` context.
  """

  @doc """
  Generate a feed.
  """
  def feed_fixture(attrs \\ %{}) do
    {:ok, feed} =
      attrs
      |> Enum.into(%{
        name: "some name",
        url: "some url"
      })
      |> Bcdn.Feeds.create_feed()

    feed
  end
end
