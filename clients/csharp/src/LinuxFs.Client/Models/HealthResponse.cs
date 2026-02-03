using System.Text.Json.Serialization;

namespace LinuxFs.Client.Models;

/// <summary>
/// Health check response.
/// </summary>
public sealed class HealthResponse
{
    [JsonPropertyName("status")]
    public string Status { get; set; } = string.Empty;
}
