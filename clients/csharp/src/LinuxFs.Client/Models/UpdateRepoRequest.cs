using System.Text.Json.Serialization;

namespace LinuxFs.Client.Models;

/// <summary>
/// Request body for updating a repository via PATCH.
/// <para>
/// For <see cref="DefaultTtlSeconds"/>: null omits the field from JSON (no change),
/// 0 clears the TTL, positive value sets a new TTL.
/// </para>
/// </summary>
public sealed class UpdateRepoRequest
{
    [JsonPropertyName("name")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Name { get; set; }

    [JsonPropertyName("max_size_bytes")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public ulong? MaxSizeBytes { get; set; }

    [JsonPropertyName("default_ttl_seconds")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public long? DefaultTtlSeconds { get; set; }

    [JsonPropertyName("tags")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public Dictionary<string, string>? Tags { get; set; }
}
