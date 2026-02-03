using System.Text.Json.Serialization;

namespace LinuxFs.Client.Models;

/// <summary>
/// Request body for creating an archive of repository contents.
/// </summary>
public sealed class ArchiveRequest
{
    [JsonPropertyName("path")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Path { get; set; }

    [JsonPropertyName("format")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? Format { get; set; }
}
