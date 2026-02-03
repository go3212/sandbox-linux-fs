using System.Text.Json.Serialization;

namespace LinuxFs.Client.Models;

/// <summary>
/// Metadata for a repository.
/// </summary>
public sealed class RepoMeta
{
    [JsonPropertyName("id")]
    public Guid Id { get; set; }

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("max_size_bytes")]
    public ulong MaxSizeBytes { get; set; }

    [JsonPropertyName("current_size_bytes")]
    public ulong CurrentSizeBytes { get; set; }

    [JsonPropertyName("file_count")]
    public ulong FileCount { get; set; }

    [JsonPropertyName("created_at")]
    public DateTimeOffset CreatedAt { get; set; }

    [JsonPropertyName("updated_at")]
    public DateTimeOffset UpdatedAt { get; set; }

    [JsonPropertyName("last_accessed_at")]
    public DateTimeOffset LastAccessedAt { get; set; }

    [JsonPropertyName("default_ttl_seconds")]
    public ulong? DefaultTtlSeconds { get; set; }

    [JsonPropertyName("tags")]
    public Dictionary<string, string> Tags { get; set; } = new();
}
