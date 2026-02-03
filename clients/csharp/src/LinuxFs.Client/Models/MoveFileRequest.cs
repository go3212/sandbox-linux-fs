using System.Text.Json.Serialization;

namespace LinuxFs.Client.Models;

/// <summary>
/// Request body for moving a file within a repository.
/// </summary>
public sealed class MoveFileRequest
{
    [JsonPropertyName("source")]
    public string Source { get; set; } = string.Empty;

    [JsonPropertyName("destination")]
    public string Destination { get; set; } = string.Empty;
}
