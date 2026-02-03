using System.Net;

namespace LinuxFs.Client.Exceptions;

/// <summary>
/// Exception thrown when the API returns an error response.
/// </summary>
public class LinuxFsApiException : LinuxFsException
{
    /// <summary>
    /// The HTTP status code returned by the server.
    /// </summary>
    public HttpStatusCode StatusCode { get; }

    /// <summary>
    /// The error code from the API error body.
    /// </summary>
    public int ErrorCode { get; }

    public LinuxFsApiException(HttpStatusCode statusCode, int errorCode, string message)
        : base(message)
    {
        StatusCode = statusCode;
        ErrorCode = errorCode;
    }
}
