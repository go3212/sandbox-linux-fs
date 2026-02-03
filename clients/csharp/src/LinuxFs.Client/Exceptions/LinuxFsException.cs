namespace LinuxFs.Client.Exceptions;

/// <summary>
/// Base exception for all linux-fs client errors.
/// </summary>
public class LinuxFsException : Exception
{
    public LinuxFsException(string message)
        : base(message)
    {
    }

    public LinuxFsException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}
