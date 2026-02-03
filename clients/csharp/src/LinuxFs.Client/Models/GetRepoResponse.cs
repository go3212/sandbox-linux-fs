using System.Text.Json.Serialization;

namespace LinuxFs.Client.Models;

/// <summary>
/// Response for getting a single repository by ID.
/// </summary>
public sealed class GetRepoResponse
{
    [JsonPropertyName("repo")]
    public RepoMeta Repo { get; set; } = new();

    [JsonPropertyName("file_count")]
    public ulong FileCount { get; set; }
}
