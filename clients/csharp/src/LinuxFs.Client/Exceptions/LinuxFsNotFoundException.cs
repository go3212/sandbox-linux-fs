using System.Net;

namespace LinuxFs.Client.Exceptions;

/// <summary>
/// Exception thrown when the requested resource is not found (HTTP 404).
/// </summary>
public class LinuxFsNotFoundException : LinuxFsApiException
{
    public LinuxFsNotFoundException(int errorCode, string message)
        : base(HttpStatusCode.NotFound, errorCode, message)
    {
    }
}
