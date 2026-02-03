using System.Text.Json.Serialization;

namespace LinuxFs.Client.Models;

/// <summary>
/// Server status information.
/// </summary>
public sealed class StatusResponse
{
    [JsonPropertyName("repo_count")]
    public ulong RepoCount { get; set; }

    [JsonPropertyName("total_size_bytes")]
    public ulong TotalSizeBytes { get; set; }

    [JsonPropertyName("uptime_seconds")]
    public long UptimeSeconds { get; set; }

    [JsonPropertyName("version")]
    public string Version { get; set; } = string.Empty;
}
