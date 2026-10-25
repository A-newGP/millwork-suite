// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.IO.Compression;
using System.Text;
using System.Text.Json;

namespace OcSpike;

/// <summary>Reading and writing the <c>.ocab</c> container, as a ZIP file or a directory.</summary>
public static class Ocab
{
    public const string MimeType = "application/vnd.opencabinet.project+zip";
    private const string FormatVersion = "0.1";

    /// <summary>Loads a project from a <c>.ocab</c> ZIP file or directory.</summary>
    public static Project Load(string path)
    {
        var entries = ReadEntries(path);
        byte[] Entry(string name) => entries.TryGetValue(name, out var bytes)
            ? bytes
            : throw new InvalidContainerException($"missing entry `{name}`");

        if (!Entry("mimetype").AsSpan().SequenceEqual(Encoding.ASCII.GetBytes(MimeType)))
        {
            throw new InvalidContainerException("wrong mimetype");
        }
        var manifest = JsonSerializer.Deserialize(Entry("manifest.json"), SpikeJsonContext.Default.Manifest)
            ?? throw new InvalidContainerException("empty manifest");
        if (manifest.FormatVersion.Split('.')[0] != "0")
        {
            throw new InvalidContainerException($"unsupported format version {manifest.FormatVersion}");
        }
        return JsonSerializer.Deserialize(Entry("project.json"), SpikeJsonContext.Default.Project)
            ?? throw new InvalidContainerException("empty project");
    }

    /// <summary>Saves a project as a <c>.ocab</c> ZIP file, <c>mimetype</c> first and uncompressed.</summary>
    public static void SaveZip(Project project, string path)
    {
        using var zip = ZipFile.Open(path, ZipArchiveMode.Create);
        using (var mimetype = zip.CreateEntry("mimetype", CompressionLevel.NoCompression).Open())
        {
            mimetype.Write(Encoding.ASCII.GetBytes(MimeType));
        }

        var manifest = new Manifest(FormatVersion, "oc-spike 0.1.0", ["project.json"]);
        using (var stream = zip.CreateEntry("manifest.json").Open())
        {
            JsonSerializer.Serialize(stream, manifest, SpikeJsonContext.Default.Manifest);
        }
        using (var stream = zip.CreateEntry("project.json").Open())
        {
            JsonSerializer.Serialize(stream, project, SpikeJsonContext.Default.Project);
        }
    }

    /// <summary>Serializes a project to canonical JSON, used to compare projects by value.</summary>
    public static string ToJson(Project project) =>
        JsonSerializer.Serialize(project, SpikeJsonContext.Default.Project);

    private static Dictionary<string, byte[]> ReadEntries(string path)
    {
        if (Directory.Exists(path))
        {
            return Directory.EnumerateFiles(path)
                .ToDictionary(file => Path.GetFileName(file), File.ReadAllBytes);
        }

        using var zip = ZipFile.OpenRead(path);
        return zip.Entries.ToDictionary(entry => entry.FullName, entry =>
        {
            using var stream = entry.Open();
            using var buffer = new MemoryStream(capacity: (int)entry.Length);
            stream.CopyTo(buffer);
            return buffer.ToArray();
        });
    }
}
