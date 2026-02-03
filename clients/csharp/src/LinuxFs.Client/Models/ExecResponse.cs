using System.Text.Json.Serialization;

namespace LinuxFs.Client.Models;

/// <summary>
/// Response from executing a command.
/// </summary>
public sealed class ExecResponse
{
    [JsonPropertyName("exit_code")]
    public int ExitCode { get; set; }

    [JsonPropertyName("stdout")]
    public string Stdout { get; set; } = string.Empty;

    [JsonPropertyName("stderr")]
    public string Stderr { get; set; } = string.Empty;

    [JsonPropertyName("duration_ms")]
    public ulong DurationMs { get; set; }

    [JsonPropertyName("truncated")]
    public bool Truncated { get; set; }
}
