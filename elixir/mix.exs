defmodule GetInBed.MixProject do
  use Mix.Project

  @version "0.1.0"
  @github_url "https://github.com/simondoesstuff/getinbed"

  def project do
    [
      app: :getinbed,
      version: @version,
      elixir: "~> 1.17",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      package: package(),
      description: "Fast batch converter for genomic interval files into normalized BED format."
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    [
      {:rustler, ">= 0.0.0", runtime: false},
      {:rustler_precompiled, "~> 0.8"}
    ]
  end

  defp package do
    [
      files: ~w[lib checksum-Elixir.GetInBed.exs mix.exs],
      licenses: ["MIT"],
      links: %{"GitHub" => @github_url}
    ]
  end
end
