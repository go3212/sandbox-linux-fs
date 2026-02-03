using System.Text.Json.Serialization;

namespace LinuxFs.Client.Models;

/// <summary>
/// Request body for creating a new repository.
/// </summary>
public sealed class CreateRepoRequest
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("max_size_bytes")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public ulong? MaxSizeBytes { get; set; }

    [JsonPropertyName("default_ttl_seconds")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public ulong? DefaultTtlSeconds { get; set; }
}
