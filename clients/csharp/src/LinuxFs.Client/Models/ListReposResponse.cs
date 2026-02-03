using System.Text.Json.Serialization;

namespace LinuxFs.Client.Models;

/// <summary>
/// Paginated list of repositories.
/// </summary>
public sealed class ListReposResponse
{
    [JsonPropertyName("repos")]
    public List<RepoMeta> Repos { get; set; } = new();

    [JsonPropertyName("page")]
    public ulong Page { get; set; }

    [JsonPropertyName("per_page")]
    public ulong PerPage { get; set; }

    [JsonPropertyName("total")]
    public ulong Total { get; set; }
}
