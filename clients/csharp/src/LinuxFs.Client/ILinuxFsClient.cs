using LinuxFs.Client.Models;

namespace LinuxFs.Client;

/// <summary>
/// Client interface for the linux-fs REST API.
/// </summary>
public interface ILinuxFsClient
{
    // Health & Status

    /// <summary>
    /// Performs a health check against the server (no auth required).
    /// </summary>
    Task<HealthResponse> HealthCheckAsync(CancellationToken ct = default);

    /// <summary>
    /// Gets the current server status.
    /// </summary>
    Task<StatusResponse> GetStatusAsync(CancellationToken ct = default);

    // Repositories

    /// <summary>
    /// Creates a new repository.
    /// </summary>
    Task<RepoMeta> CreateRepoAsync(CreateRepoRequest request, CancellationToken ct = default);

    /// <summary>
    /// Lists repositories with optional pagination and sorting.
    /// </summary>
    Task<ListReposResponse> ListReposAsync(int? page = null, int? perPage = null, string? sort = null, CancellationToken ct = default);

    /// <summary>
    /// Gets a single repository by ID.
    /// </summary>
    Task<GetRepoResponse> GetRepoAsync(Guid repoId, CancellationToken ct = default);

    /// <summary>
    /// Updates a repository via PATCH.
    /// </summary>
    Task<RepoMeta> UpdateRepoAsync(Guid repoId, UpdateRepoRequest request, CancellationToken ct = default);

    /// <summary>
    /// Deletes a repository.
    /// </summary>
    Task DeleteRepoAsync(Guid repoId, CancellationToken ct = default);

    // Files

    /// <summary>
    /// Lists files in a repository with optional filtering and pagination.
    /// </summary>
    Task<ListFilesResponse> ListFilesAsync(Guid repoId, string? prefix = null, bool? recursive = null, int? page = null, int? perPage = null, CancellationToken ct = default);

    /// <summary>
    /// Uploads a file from a byte array.
    /// </summary>
    Task<FileMeta> UploadFileAsync(Guid repoId, string path, byte[] content, long? ttlSeconds = null, CancellationToken ct = default);

    /// <summary>
    /// Uploads a file from a stream.
    /// </summary>
    Task<FileMeta> UploadFileAsync(Guid repoId, string path, Stream content, long? ttlSeconds = null, CancellationToken ct = default);

    /// <summary>
    /// Downloads a file as a stream. Returns the response body stream.
    /// </summary>
    Task<Stream> DownloadFileAsync(Guid repoId, string path, string? ifNoneMatch = null, CancellationToken ct = default);

    /// <summary>
    /// Performs a HEAD request on a file and returns parsed headers.
    /// </summary>
    Task<FileHeadResponse> HeadFileAsync(Guid repoId, string path, CancellationToken ct = default);

    /// <summary>
    /// Deletes a file from a repository.
    /// </summary>
    Task DeleteFileAsync(Guid repoId, string path, CancellationToken ct = default);

    /// <summary>
    /// Moves a file within a repository.
    /// </summary>
    Task<FileMeta> MoveFileAsync(Guid repoId, MoveFileRequest request, CancellationToken ct = default);

    /// <summary>
    /// Copies a file within a repository.
    /// </summary>
    Task<FileMeta> CopyFileAsync(Guid repoId, CopyFileRequest request, CancellationToken ct = default);

    // Shell

    /// <summary>
    /// Executes a command in the context of a repository.
    /// </summary>
    Task<ExecResponse> ExecAsync(Guid repoId, ExecRequest request, CancellationToken ct = default);

    // Archive

    /// <summary>
    /// Creates an archive of repository contents and returns the binary stream.
    /// </summary>
    Task<Stream> CreateArchiveAsync(Guid repoId, ArchiveRequest request, CancellationToken ct = default);
}
