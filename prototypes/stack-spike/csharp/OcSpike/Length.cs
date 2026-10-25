// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Globalization;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace OcSpike;

/// <summary>An exact signed length in nanometers (ADR-0009).</summary>
[JsonConverter(typeof(LengthJsonConverter))]
public readonly record struct Length(long Nm) : IComparable<Length>
{
    private const long NmPerInch = 25_400_000;

    public static readonly Length Zero = new(0);

    public static Length operator +(Length a, Length b) => new(checked(a.Nm + b.Nm));

    public static Length operator -(Length a, Length b) => new(checked(a.Nm - b.Nm));

    public static Length operator *(Length a, long factor) => new(checked(a.Nm * factor));

    public static bool operator <(Length a, Length b) => a.Nm < b.Nm;

    public static bool operator >(Length a, Length b) => a.Nm > b.Nm;

    public static bool operator <=(Length a, Length b) => a.Nm <= b.Nm;

    public static bool operator >=(Length a, Length b) => a.Nm >= b.Nm;

    public int CompareTo(Length other) => Nm.CompareTo(other.Nm);

    /// <summary>Floor of half the length.</summary>
    public Length Half() => new(Nm >= 0 ? Nm / 2 : (Nm - 1) / 2);

    /// <summary>
    /// Splits the length into <paramref name="parts"/> lengths that differ by at
    /// most 1 nm and add up exactly; the first ones receive the remainder.
    /// </summary>
    public IReadOnlyList<Length> SplitEvenly(int parts)
    {
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(parts);
        var baseNm = Nm / parts;
        var remainder = Nm % parts;
        if (remainder < 0)
        {
            baseNm -= 1;
            remainder += parts;
        }
        return [.. Enumerable.Range(0, parts).Select(i => new Length(baseNm + (i < remainder ? 1 : 0)))];
    }

    /// <summary>
    /// Formats as fractional inches rounded to the nearest 1/<paramref name="denominator"/>,
    /// ties away from zero: <c>30"</c>, <c>7/32"</c>, <c>23-7/32"</c>.
    /// </summary>
    public string ToFractionalInches(int denominator)
    {
        var units = RoundHalfAway((Int128)Nm * denominator, NmPerInch);
        var sign = units < 0 ? "-" : "";
        var (whole, rest) = Int128.DivRem(Int128.Abs(units), denominator);
        if (rest == 0)
        {
            return string.Create(CultureInfo.InvariantCulture, $"{sign}{whole}\"");
        }
        var numerator = (int)rest;
        var common = Gcd(numerator, denominator);
        var fraction = string.Create(CultureInfo.InvariantCulture, $"{numerator / common}/{denominator / common}");
        return whole == 0
            ? $"{sign}{fraction}\""
            : string.Create(CultureInfo.InvariantCulture, $"{sign}{whole}-{fraction}\"");
    }

    private static int Gcd(int a, int b)
    {
        while (b != 0)
        {
            (a, b) = (b, a % b);
        }
        return a;
    }

    public override string ToString() => Nm.ToString(CultureInfo.InvariantCulture);

    private static Int128 RoundHalfAway(Int128 numerator, Int128 denominator)
    {
        var negative = (numerator < 0) != (denominator < 0);
        var (n, d) = (Int128.Abs(numerator), Int128.Abs(denominator));
        var magnitude = ((2 * n) + d) / (2 * d);
        return negative ? -magnitude : magnitude;
    }
}

/// <summary>Reads and writes a <see cref="Length"/> as a JSON integer.</summary>
public sealed class LengthJsonConverter : JsonConverter<Length>
{
    public override Length Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) =>
        new(reader.GetInt64());

    public override void Write(Utf8JsonWriter writer, Length value, JsonSerializerOptions options)
    {
        ArgumentNullException.ThrowIfNull(writer);
        writer.WriteNumberValue(value.Nm);
    }
}
