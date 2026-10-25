// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json.Serialization;

namespace OcSpike;

// The subset of the RFC-0001 data model used by the spike.

public sealed record Project(
    string Id,
    string Name,
    Settings Settings,
    Library Library,
    IReadOnlyList<Product> Products);

public sealed record Settings(string Units, int PrecisionDenominator);

public sealed record Library(
    IReadOnlyList<Material> Materials,
    IReadOnlyList<EdgeBand> EdgeBands,
    IReadOnlyList<Standard> Standards);

public sealed record Material(
    string Id,
    string Name,
    Length NominalThickness,
    Length ActualThickness,
    Length SheetLength,
    Length SheetWidth,
    string Grain);

public sealed record EdgeBand(string Id, string Name, Length Thickness, Length Width);

public sealed record Standard(
    string Id,
    string Name,
    int Version,
    string Method,
    ToeKick ToeKick,
    Back Back,
    Stretchers Stretchers,
    Fronts Fronts,
    Hinges Hinges);

public sealed record ToeKick(Length Height);

public sealed record Back(Length GrooveDepth);

public sealed record Stretchers(Length Width);

public sealed record Fronts(Length Gap);

public sealed record Hinges(
    Length CupDiameter,
    Length CupDepth,
    Length EdgeDistance,
    Length EndDistance,
    Length ThirdHingeAbove);

public sealed record Product(
    string Id,
    string Label,
    string Type,
    string Standard,
    Length Width,
    Length Height,
    Length Depth,
    int Quantity,
    MaterialRoles Materials,
    FrontNode Front);

public sealed record MaterialRoles(
    string Case,
    string Back,
    string Doors,
    string EdgeBandCase,
    string EdgeBandFronts);

public sealed record FrontNode(string Id, Insert Insert);

[JsonPolymorphic(TypeDiscriminatorPropertyName = "kind")]
[JsonDerivedType(typeof(DoorsInsert), "doors")]
public abstract record Insert;

public sealed record DoorsInsert(
    int Count,
    string Style,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] HingeSide? HingeSide = null) : Insert;

[JsonConverter(typeof(JsonStringEnumConverter<HingeSide>))]
public enum HingeSide
{
    [JsonStringEnumMemberName("left")] Left,
    [JsonStringEnumMemberName("right")] Right,
}

/// <summary>Compile-time JSON serialization, required for native AOT.</summary>
[JsonSourceGenerationOptions(
    PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase,
    WriteIndented = true,
    RespectNullableAnnotations = true,
    RespectRequiredConstructorParameters = true)]
[JsonSerializable(typeof(Project))]
[JsonSerializable(typeof(Manifest))]
internal sealed partial class SpikeJsonContext : JsonSerializerContext;

internal sealed record Manifest(
    string FormatVersion,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? Generator = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] IReadOnlyList<string>? Entries = null);
