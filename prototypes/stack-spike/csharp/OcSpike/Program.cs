// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Diagnostics;
using System.Globalization;
using System.Text.Json;
using OcSpike;

const string Usage = """
    usage: oc-spike evaluate <input> <output-dir>
           oc-spike roundtrip <input> <temp.ocab>
           oc-spike bench <input> <iterations>
    """;

try
{
    return args switch
    {
        ["evaluate", var input, var output] => Evaluate(input, output),
        ["roundtrip", var input, var temp] => Roundtrip(input, temp),
        ["bench", var input, var n] when int.TryParse(n, CultureInfo.InvariantCulture, out var iterations)
            && iterations > 0 => Bench(input, iterations),
        _ => PrintUsage(),
    };
}
catch (Exception e) when (e is InvalidContainerException or InvalidProjectException or IOException
                              or JsonException or InvalidDataException)
{
    Console.Error.WriteLine($"error: {e.Message}");
    return 2;
}

static int PrintUsage()
{
    Console.Error.WriteLine(Usage);
    return 1;
}

static int Evaluate(string input, string output)
{
    var evaluation = Evaluator.Evaluate(Ocab.Load(input));
    Directory.CreateDirectory(output);
    File.WriteAllText(Path.Combine(output, "cutlist.csv"), Report.CutList(evaluation));
    File.WriteAllText(Path.Combine(output, "operations.csv"), Report.Operations(evaluation));
    return 0;
}

static int Roundtrip(string input, string temp)
{
    var original = Ocab.Load(input);
    File.Delete(temp);
    Ocab.SaveZip(original, temp);
    var reopened = Ocab.Load(temp);

    var same = Ocab.ToJson(original) == Ocab.ToJson(reopened)
        && Report.CutList(Evaluator.Evaluate(original)) == Report.CutList(Evaluator.Evaluate(reopened))
        && Report.Operations(Evaluator.Evaluate(original)) == Report.Operations(Evaluator.Evaluate(reopened));
    if (!same)
    {
        Console.Error.WriteLine("round trip mismatch");
        return 3;
    }
    Console.WriteLine("round trip OK");
    return 0;
}

static int Bench(string input, int iterations)
{
    var project = Ocab.Load(input);
    var sink = 0;
    var watch = Stopwatch.StartNew();
    for (var i = 0; i < iterations; i++)
    {
        sink += Evaluator.Evaluate(project).Parts.Count;
    }
    var perIteration = watch.Elapsed.TotalNanoseconds / iterations;
    Console.WriteLine(string.Create(CultureInfo.InvariantCulture,
        $"{perIteration:F0} ns per evaluation ({iterations} iterations, {sink} parts)"));
    return 0;
}
