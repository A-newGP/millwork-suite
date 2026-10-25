// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Xunit;

namespace OcSpike.Tests;

public class LengthTests
{
    [Theory]
    [InlineData(762_000_000, "30\"")]
    [InlineData(5_556_250, "7/32\"")]
    [InlineData(589_756_250, "23-7/32\"")]
    [InlineData(0, "0\"")]
    [InlineData(-5_556_250, "-7/32\"")]
    public void FormatsWholeFractionAndMixedInches(long nm, string expected) =>
        Assert.Equal(expected, new Length(nm).ToFractionalInches(32));

    [Theory]
    [InlineData(396_875, "1/32\"")] // 1/64" is exactly half of 1/32".
    [InlineData(-396_875, "-1/32\"")]
    [InlineData(396_874, "0\"")]
    public void RoundsTiesAwayFromZero(long nm, string expected) =>
        Assert.Equal(expected, new Length(nm).ToFractionalInches(32));

    [Fact]
    public void SplitEvenlyDistributesTheRemainderFromTheLeft() =>
        Assert.Equal(
            [new Length(390_723_438), new Length(390_723_437)],
            new Length(781_446_875).SplitEvenly(2));

    [Fact]
    public void SplitEvenlyAlwaysAddsUp()
    {
        foreach (var total in new long[] { 0, 1, 7, 999_999_999, 781_446_875 })
        {
            for (var parts = 1; parts <= 7; parts++)
            {
                Assert.Equal(total, new Length(total).SplitEvenly(parts).Sum(p => p.Nm));
            }
        }
    }
}
