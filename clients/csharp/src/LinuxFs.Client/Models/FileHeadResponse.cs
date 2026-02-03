namespace LinuxFs.Client.Models;

/// <summary>
/// Parsed response headers from a HEAD request on a file.
/// </summary>
public sealed class FileHeadResponse
{
    /// <summary>
    /// The MIME content type of the file.
    /// </summary>
    public string ContentType { get; set; } = string.Empty;

    /// <summary>
    /// The file size in bytes.
    /// </summary>
    public long ContentLength { get; set; }

    /// <summary>
    /// The ETag for conditional requests.
    /// </summary>
    public string? Etag { get; set; }

    /// <summary>
    /// When the file was last modified.
    /// </summary>
    public DateTimeOffset? LastModified { get; set; }
}
