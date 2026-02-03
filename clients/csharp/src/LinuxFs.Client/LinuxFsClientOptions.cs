namespace LinuxFs.Client;

/// <summary>
/// Configuration options for the LinuxFs client.
/// </summary>
public sealed class LinuxFsClientOptions
{
    /// <summary>
    /// The base URL of the linux-fs server (e.g., "https://localhost:8080").
    /// </summary>
    public string BaseUrl { get; set; } = string.Empty;

    /// <summary>
    /// The API key used for authentication via the X-API-Key header.
    /// </summary>
    public string ApiKey { get; set; } = string.Empty;
}
