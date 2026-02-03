using System.Text.Json.Serialization;

namespace LinuxFs.Client.Models;

/// <summary>
/// Metadata for a file within a repository.
/// </summary>
public sealed class FileMeta
{
    [JsonPropertyName("repo_id")]
    public Guid RepoId { get; set; }

    [JsonPropertyName("path")]
    public string Path { get; set; } = string.Empty;

    [JsonPropertyName("size_bytes")]
    public ulong SizeBytes { get; set; }

    [JsonPropertyName("etag")]
    public string Etag { get; set; } = string.Empty;

    [JsonPropertyName("content_type")]
    public string ContentType { get; set; } = string.Empty;

    [JsonPropertyName("created_at")]
    public DateTimeOffset CreatedAt { get; set; }

    [JsonPropertyName("updated_at")]
    public DateTimeOffset UpdatedAt { get; set; }

    [JsonPropertyName("last_accessed_at")]
    public DateTimeOffset LastAccessedAt { get; set; }

    [JsonPropertyName("access_count")]
    public ulong AccessCount { get; set; }

    [JsonPropertyName("expires_at")]
    public DateTimeOffset? ExpiresAt { get; set; }
}
