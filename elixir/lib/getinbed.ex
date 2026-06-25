defmodule GetInBed do
  use RustlerPrecompiled,
    otp_app: :getinbed,
    crate: :getinbed,
    base_url:
      "https://github.com/simondoesstuff/getinbed/releases/download/v#{Mix.Project.config()[:version]}",
    version: Mix.Project.config()[:version],
    nif_versions: ["2.16"],
    targets: ~w[
      aarch64-apple-darwin
      x86_64-apple-darwin
      x86_64-unknown-linux-gnu
      aarch64-unknown-linux-gnu
      x86_64-unknown-linux-musl
    ],
    force_build: System.get_env("GETINBED_BUILD") in ["1", "true"],
    path: "..",
    features: ["nif"]

  @doc """
  Process a batch of genomic interval files into normalised BED format.

  ## Arguments

  - `files` — list of absolute paths to input files
  - `opts` — keyword list of options:
    - `out: path` — output directory (default: same directory as each input file)
    - `format: atom` — force input format, e.g. `:bed`, `:gff3`, `:vcf`
    - `extra_columns: [integer]` — 0-indexed source columns to append after chrom/start/end
    - `blacklist: path` — BED file of regions to subtract
    - `split_on: integer` — 0-indexed column to split output on
    - `chroms: [string]` — whitelist of chromosome names to keep; omit to keep all
    - `no_clean: boolean` — skip deduplication and malformed-row removal
    - `no_sort: boolean` — skip sorting

  ## Returns

  `{:ok, [output_path]}` or `{:error, reason}`.
  """
  @spec to_bed([binary()], keyword()) :: {:ok, [binary()]} | {:error, binary()}
  def to_bed(_files, _opts), do: :erlang.nif_error(:nif_not_loaded)
end
