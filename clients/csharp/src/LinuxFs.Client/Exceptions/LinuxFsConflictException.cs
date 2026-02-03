using System.Net;

namespace LinuxFs.Client.Exceptions;

/// <summary>
/// Exception thrown when the request conflicts with the current state (HTTP 409).
/// </summary>
public class LinuxFsConflictException : LinuxFsApiException
{
    public LinuxFsConflictException(int errorCode, string message)
        : base(HttpStatusCode.Conflict, errorCode, message)
    {
    }
}
