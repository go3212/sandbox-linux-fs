using System.Text.Json.Serialization;

namespace LinuxFs.Client.Models;

/// <summary>
/// Request body for executing a command in a repository context.
/// </summary>
public sealed class ExecRequest
{
    [JsonPropertyName("command")]
    public string Command { get; set; } = string.Empty;

    [JsonPropertyName("args")]
    public List<string> Args { get; set; } = new();

    [JsonPropertyName("timeout_seconds")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public ulong? TimeoutSeconds { get; set; }

    [JsonPropertyName("max_output_bytes")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public ulong? MaxOutputBytes { get; set; }
}
