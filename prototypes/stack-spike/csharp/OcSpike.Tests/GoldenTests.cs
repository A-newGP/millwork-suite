// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Xunit;

namespace OcSpike.Tests;

/// <summary>The spike's acceptance tests: exact golden outputs and a lossless round trip.</summary>
public class GoldenTests
{
    private static readonly string SpikeDir = FindSpikeDir();
    private static readonly string Fixture = Path.Combine(SpikeDir, "fixtures", "sample.ocab");

    [Fact]
    public void CutListMatchesGoldenOutput() =>
        Assert.Equal(Expected("cutlist.csv"), Report.CutList(Evaluator.Evaluate(Ocab.Load(Fixture))));

    [Fact]
    public void OperationsMatchGoldenOutput() =>
        Assert.Equal(Expected("operations.csv"), Report.Operations(Evaluator.Evaluate(Ocab.Load(Fixture))));

    [Fact]
    public void ZipRoundTripPreservesProjectAndResults()
    {
        var original = Ocab.Load(Fixture);
        var temp = Path.Combine(Path.GetTempPath(), $"oc-spike-{Guid.NewGuid():N}.ocab");
        try
        {
            Ocab.SaveZip(original, temp);
            var reopened = Ocab.Load(temp);
            Assert.Equal(Ocab.ToJson(original), Ocab.ToJson(reopened));
            Assert.Equal(
                Report.CutList(Evaluator.Evaluate(original)),
                Report.CutList(Evaluator.Evaluate(reopened)));
        }
        finally
        {
            File.Delete(temp);
        }
    }

    [Fact]
    public void RejectsUnknownMajorFormatVersion()
    {
        var dir = Path.Combine(Path.GetTempPath(), $"oc-spike-v9-{Guid.NewGuid():N}.ocab");
        Directory.CreateDirectory(dir);
        try
        {
            foreach (var entry in new[] { "mimetype", "project.json" })
            {
                File.Copy(Path.Combine(Fixture, entry), Path.Combine(dir, entry));
            }
            File.WriteAllText(Path.Combine(dir, "manifest.json"), """{"formatVersion": "9.0"}""");
            Assert.Throws<InvalidContainerException>(() => Ocab.Load(dir));
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    private static string Expected(string name) => File.ReadAllText(Path.Combine(SpikeDir, "expected", name));

    private static string FindSpikeDir()
    {
        for (var dir = new DirectoryInfo(AppContext.BaseDirectory); dir is not null; dir = dir.Parent)
        {
            if (Directory.Exists(Path.Combine(dir.FullName, "fixtures", "sample.ocab")))
            {
                return dir.FullName;
            }
        }
        throw new DirectoryNotFoundException("stack spike directory not found");
    }
}
