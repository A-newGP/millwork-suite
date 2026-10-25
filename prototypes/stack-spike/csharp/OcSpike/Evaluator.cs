// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace OcSpike;

/// <summary>A derived part; edges are banded in the order x0, x1, y0, y1.</summary>
public sealed record Part(
    string Product,
    string Key,
    Material Material,
    int Quantity,
    Length Length,
    Length Width,
    EdgeBand?[] Edges)
{
    public Length Thickness => Material.ActualThickness;

    public Length CutLength => Length - BandThickness(Edges[0]) - BandThickness(Edges[1]);

    public Length CutWidth => Width - BandThickness(Edges[2]) - BandThickness(Edges[3]);

    private static Length BandThickness(EdgeBand? band) => band?.Thickness ?? Length.Zero;
}

public sealed record Hole(string Product, string Part, Length X, Length Y, Length Diameter, Length Depth);

public sealed record Evaluation(IReadOnlyList<Part> Parts, IReadOnlyList<Hole> Holes);

/// <summary>
/// Deterministic evaluation of products into parts and operations, following
/// the rules in the spike README.
/// </summary>
public static class Evaluator
{
    public static Evaluation Evaluate(Project project)
    {
        ArgumentNullException.ThrowIfNull(project);
        var library = new LibraryIndex(project.Library);
        var parts = new List<Part>();
        var holes = new List<Hole>();
        foreach (var product in project.Products)
        {
            EvaluateProduct(product, library, parts, holes);
        }
        return new Evaluation(parts, holes);
    }

    private static void EvaluateProduct(Product product, LibraryIndex lib, List<Part> parts, List<Hole> holes)
    {
        var std = lib.Standard(product.Standard, product);
        var roles = product.Materials;
        var caseMaterial = lib.Material(roles.Case, product);
        var back = lib.Material(roles.Back, product);
        var doors = lib.Material(roles.Doors, product);
        var caseBand = lib.EdgeBand(roles.EdgeBandCase, product);
        var frontBand = lib.EdgeBand(roles.EdgeBandFronts, product);

        var (w, d) = (product.Width, product.Depth);
        var t = caseMaterial.ActualThickness;
        var boxHeight = product.Height - std.ToeKick.Height;
        var gap = std.Fronts.Gap;
        var groove = std.Back.GrooveDepth;
        var innerWidth = w - (t * 2);

        EdgeBand?[] FrontY1() => [null, null, null, caseBand];
        EdgeBand?[] Unbanded() => [null, null, null, null];
        Part NewPart(string key, Material material, Length length, Length width, EdgeBand?[] edges) =>
            new(product.Label, key, material, product.Quantity, length, width, edges);

        var productParts = new List<Part>
        {
            NewPart("case/side.left", caseMaterial, boxHeight, d, FrontY1()),
            NewPart("case/side.right", caseMaterial, boxHeight, d, FrontY1()),
            NewPart("case/bottom", caseMaterial, innerWidth, d, FrontY1()),
            NewPart("case/stretcher.front", caseMaterial, innerWidth, std.Stretchers.Width, FrontY1()),
            NewPart("case/stretcher.back", caseMaterial, innerWidth, std.Stretchers.Width, Unbanded()),
            NewPart("case/back", back, boxHeight - t + groove, innerWidth + (groove * 2), Unbanded()),
        };

        if (product.Front.Insert is not DoorsInsert insert)
        {
            throw new InvalidProjectException($"{product.Label}: unsupported front insert");
        }
        HingeSide[] hingeSides = insert.Count switch
        {
            1 => [insert.HingeSide ?? HingeSide.Left],
            2 => [HingeSide.Left, HingeSide.Right],
            _ => throw new InvalidProjectException($"{product.Label}: unsupported door count {insert.Count}"),
        };
        var doorLength = boxHeight - gap;
        var doorWidths = (w - gap - (gap * (insert.Count - 1))).SplitEvenly(insert.Count);

        for (var i = 0; i < doorWidths.Count; i++)
        {
            var key = $"front/door.{i + 1}";
            holes.AddRange(HingeHoles(product, std.Hinges, key, doorLength, doorWidths[i], hingeSides[i]));
            productParts.Add(NewPart(key, doors, doorLength, doorWidths[i], [frontBand, frontBand, frontBand, frontBand]));
        }

        var bad = productParts.Find(p => p.Length <= Length.Zero || p.Width <= Length.Zero);
        if (bad is not null)
        {
            throw new InvalidProjectException($"{product.Label}: non-positive size for {bad.Key}");
        }
        parts.AddRange(productParts);
    }

    private static IEnumerable<Hole> HingeHoles(
        Product product, Hinges hinges, string key, Length doorLength, Length doorWidth, HingeSide side)
    {
        var y = side == HingeSide.Left ? hinges.EdgeDistance : doorWidth - hinges.EdgeDistance;
        var xs = new List<Length> { hinges.EndDistance, doorLength - hinges.EndDistance };
        if (doorLength > hinges.ThirdHingeAbove)
        {
            xs.Add(doorLength.Half());
        }
        xs.Sort();
        return xs.Select(x => new Hole(product.Label, key, x, y, hinges.CupDiameter, hinges.CupDepth));
    }

    private sealed class LibraryIndex(Library library)
    {
        private readonly Dictionary<string, Material> _materials = library.Materials.ToDictionary(m => m.Id);
        private readonly Dictionary<string, EdgeBand> _edgeBands = library.EdgeBands.ToDictionary(b => b.Id);
        private readonly Dictionary<string, Standard> _standards = library.Standards.ToDictionary(s => s.Id);

        public Material Material(string id, Product product) => Get(_materials, id, product);

        public EdgeBand EdgeBand(string id, Product product) => Get(_edgeBands, id, product);

        public Standard Standard(string id, Product product) => Get(_standards, id, product);

        private static T Get<T>(Dictionary<string, T> map, string id, Product product) =>
            map.TryGetValue(id, out var value)
                ? value
                : throw new InvalidProjectException($"{product.Label}: unknown reference {id}");
    }
}
