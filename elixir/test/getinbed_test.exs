defmodule GetInBedTest do
  use ExUnit.Case

  setup do
    out = Path.join(System.tmp_dir!(), "getinbed_#{:erlang.unique_integer([:positive])}")
    File.mkdir_p!(out)
    on_exit(fn -> File.rm_rf!(out) end)
    {:ok, out: out}
  end

  defp write_bed(dir, name, content) do
    path = Path.join(dir, name)
    File.write!(path, content)
    path
  end

  test "basic roundtrip", %{out: out} do
    input = write_bed(out, "a.bed", "chr1\t100\t200\n")
    assert {:ok, [out_path]} = GetInBed.to_bed([input], out: out)
    assert String.trim(File.read!(out_path)) == "chr1\t100\t200"
  end

  test "sorts output karyotypically", %{out: out} do
    input = write_bed(out, "a.bed", "chr2\t0\t100\nchr1\t500\t600\nchr1\t0\t100\n")
    assert {:ok, [out_path]} = GetInBed.to_bed([input], out: out)
    lines = out_path |> File.read!() |> String.split("\n", trim: true)
    assert lines == ["chr1\t0\t100", "chr1\t500\t600", "chr2\t0\t100"]
  end

  test "deduplicates exact records", %{out: out} do
    input = write_bed(out, "a.bed", "chr1\t0\t100\nchr1\t0\t100\nchr1\t100\t200\n")
    assert {:ok, [out_path]} = GetInBed.to_bed([input], out: out)
    lines = out_path |> File.read!() |> String.split("\n", trim: true)
    assert length(lines) == 2
  end

  test "drops malformed rows (start >= end)", %{out: out} do
    input = write_bed(out, "a.bed", "chr1\t200\t100\nchr1\t0\t100\n")
    assert {:ok, [out_path]} = GetInBed.to_bed([input], out: out)
    lines = out_path |> File.read!() |> String.split("\n", trim: true)
    assert lines == ["chr1\t0\t100"]
  end

  test "skips # comments and track/browser headers", %{out: out} do
    content = "# comment\ntrack name=test\nbrowser position chr1:1-100\nchr1\t0\t100\n"
    input = write_bed(out, "a.bed", content)
    assert {:ok, [out_path]} = GetInBed.to_bed([input], out: out)
    lines = out_path |> File.read!() |> String.split("\n", trim: true)
    assert lines == ["chr1\t0\t100"]
  end

  test "chrom whitelist filters unwanted contigs", %{out: out} do
    input = write_bed(out, "a.bed", "chr1\t0\t100\nscaffold_1\t0\t50\nchrUn\t0\t10\n")
    assert {:ok, [out_path]} = GetInBed.to_bed([input], out: out, chroms: ["chr1", "chr2"])
    lines = out_path |> File.read!() |> String.split("\n", trim: true)
    assert lines == ["chr1\t0\t100"]
  end

  test "no filter when chroms not specified", %{out: out} do
    input = write_bed(out, "a.bed", "chr1\t0\t100\nscaffold_1\t0\t50\n")
    assert {:ok, [out_path]} = GetInBed.to_bed([input], out: out)
    lines = out_path |> File.read!() |> String.split("\n", trim: true)
    assert length(lines) == 2
  end

  test "extra_columns appended after chrom/start/end", %{out: out} do
    input = write_bed(out, "a.bed", "chr1\t0\t100\tpeak1\t500\t+\n")
    assert {:ok, [out_path]} = GetInBed.to_bed([input], out: out, extra_columns: [3, 5])
    assert String.trim(File.read!(out_path)) == "chr1\t0\t100\tpeak1\t+"
  end

  test "no_sort preserves input order", %{out: out} do
    input = write_bed(out, "a.bed", "chr2\t0\t100\nchr1\t0\t100\n")
    assert {:ok, [out_path]} = GetInBed.to_bed([input], out: out, no_sort: true)
    lines = out_path |> File.read!() |> String.split("\n", trim: true)
    assert hd(lines) == "chr2\t0\t100"
  end

  test "batch processes multiple files", %{out: out} do
    f1 = write_bed(out, "a.bed", "chr1\t0\t100\n")
    f2 = write_bed(out, "b.bed", "chr2\t0\t200\n")
    assert {:ok, paths} = GetInBed.to_bed([f1, f2], out: out)
    assert length(paths) == 2
    Enum.each(paths, fn p -> assert File.exists?(p) end)
  end

  test "returns error for unknown format", %{out: out} do
    path = Path.join(out, "test.unknownxyz")
    File.write!(path, "data\n")
    assert {:error, _reason} = GetInBed.to_bed([path], out: out)
  end
end
