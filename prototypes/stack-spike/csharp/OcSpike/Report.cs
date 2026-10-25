// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Globalization;
using System.Text;

namespace OcSpike;

/// <summary>CSV reports with the exact layout defined in the spike README.</summary>
public static class Report
{
    private const int Denominator = 32;

    public static string CutList(Evaluation evaluation)
    {
        ArgumentNullException.ThrowIfNull(evaluation);
        var csv = new StringBuilder(
            "product,part,material,qty,length,width,thickness,length_nm,width_nm,thickness_nm," +
            "cut_length_nm,cut_width_nm,band_x0,band_x1,band_y0,band_y1\n");
        foreach (var p in evaluation.Parts)
        {
            var bands = string.Join(',', p.Edges.Select(e => e?.Name ?? ""));
            csv.Append(CultureInfo.InvariantCulture,
                $"{p.Product},{p.Key},{p.Material.Name},{p.Quantity}," +
                $"{p.Length.ToFractionalInches(Denominator)},{p.Width.ToFractionalInches(Denominator)}," +
                $"{p.Thickness.ToFractionalInches(Denominator)},{p.Length},{p.Width},{p.Thickness}," +
                $"{p.CutLength},{p.CutWidth},{bands}\n");
        }
        return csv.ToString();
    }

    public static string Operations(Evaluation evaluation)
    {
        ArgumentNullException.ThrowIfNull(evaluation);
        var csv = new StringBuilder("product,part,face,kind,x_nm,y_nm,diameter_nm,depth_nm\n");
        foreach (var h in evaluation.Holes)
        {
            csv.Append(CultureInfo.InvariantCulture,
                $"{h.Product},{h.Part},B,hole,{h.X},{h.Y},{h.Diameter},{h.Depth}\n");
        }
        return csv.ToString();
    }
}
