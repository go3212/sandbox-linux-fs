using System.Text.Json.Serialization;

namespace LinuxFs.Client.Models;

/// <summary>
/// Paginated list of files in a repository.
/// </summary>
public sealed class ListFilesResponse
{
    [JsonPropertyName("files")]
    public List<FileMeta> Files { get; set; } = new();

    [JsonPropertyName("page")]
    public ulong Page { get; set; }

    [JsonPropertyName("per_page")]
    public ulong PerPage { get; set; }
}
