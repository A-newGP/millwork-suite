// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace OcSpike;

/// <summary>The container is not a readable <c>.ocab</c>.</summary>
public sealed class InvalidContainerException(string message) : Exception(message);

/// <summary>The project cannot be evaluated.</summary>
public sealed class InvalidProjectException(string message) : Exception(message);
