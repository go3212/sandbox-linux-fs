using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using System.Web;
using LinuxFs.Client.Exceptions;
using LinuxFs.Client.Models;

namespace LinuxFs.Client;

/// <summary>
/// HTTP client implementation for the linux-fs REST API.
/// </summary>
public sealed class LinuxFsClient : ILinuxFsClient
{
    private readonly HttpClient _httpClient;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull,
    };

    /// <summary>
    /// Creates a new LinuxFsClient using the provided HttpClient.
    /// The HttpClient should have its BaseAddress configured.
    /// </summary>
    public LinuxFsClient(HttpClient httpClient)
    {
        _httpClient = httpClient ?? throw new ArgumentNullException(nameof(httpClient));
    }

    // ──────────────────── Health & Status ────────────────────

    public async Task<HealthResponse> HealthCheckAsync(CancellationToken ct = default)
    {
        using var request = new HttpRequestMessage(HttpMethod.Get, "/health");
        using var response = await _httpClient.SendAsync(request, ct).ConfigureAwait(false);

        response.EnsureSuccessStatusCode();

        var health = await DeserializeAsync<HealthResponse>(response, ct).ConfigureAwait(false);
        return health ?? throw new LinuxFsException("Failed to deserialize health response.");
    }

    public async Task<StatusResponse> GetStatusAsync(CancellationToken ct = default)
    {
        return await SendAndDeserializeAsync<StatusResponse>(HttpMethod.Get, "/api/v1/status", ct: ct)
            .ConfigureAwait(false);
    }

    // ──────────────────── Repositories ────────────────────

    public async Task<RepoMeta> CreateRepoAsync(CreateRepoRequest request, CancellationToken ct = default)
    {
        return await SendAndDeserializeAsync<RepoMeta>(
            HttpMethod.Post,
            "/api/v1/repos",
            JsonContent(request),
            ct
        ).ConfigureAwait(false);
    }

    public async Task<ListReposResponse> ListReposAsync(
        int? page = null, int? perPage = null, string? sort = null, CancellationToken ct = default)
    {
        var query = BuildQuery(
            ("page", page?.ToString()),
            ("per_page", perPage?.ToString()),
            ("sort", sort)
        );

        return await SendAndDeserializeAsync<ListReposResponse>(
            HttpMethod.Get,
            $"/api/v1/repos{query}",
            ct: ct
        ).ConfigureAwait(false);
    }

    public async Task<GetRepoResponse> GetRepoAsync(Guid repoId, CancellationToken ct = default)
    {
        return await SendAndDeserializeAsync<GetRepoResponse>(
            HttpMethod.Get,
            $"/api/v1/repos/{repoId}",
            ct: ct
        ).ConfigureAwait(false);
    }

    public async Task<RepoMeta> UpdateRepoAsync(Guid repoId, UpdateRepoRequest request, CancellationToken ct = default)
    {
        return await SendAndDeserializeAsync<RepoMeta>(
            HttpMethod.Patch,
            $"/api/v1/repos/{repoId}",
            JsonContent(request),
            ct
        ).ConfigureAwait(false);
    }

    public async Task DeleteRepoAsync(Guid repoId, CancellationToken ct = default)
    {
        await SendNoContentAsync(HttpMethod.Delete, $"/api/v1/repos/{repoId}", ct: ct)
            .ConfigureAwait(false);
    }

    // ──────────────────── Files ────────────────────

    public async Task<ListFilesResponse> ListFilesAsync(
        Guid repoId,
        string? prefix = null,
        bool? recursive = null,
        int? page = null,
        int? perPage = null,
        CancellationToken ct = default)
    {
        var query = BuildQuery(
            ("prefix", prefix),
            ("recursive", recursive?.ToString().ToLowerInvariant()),
            ("page", page?.ToString()),
            ("per_page", perPage?.ToString())
        );

        return await SendAndDeserializeAsync<ListFilesResponse>(
            HttpMethod.Get,
            $"/api/v1/repos/{repoId}/files{query}",
            ct: ct
        ).ConfigureAwait(false);
    }

    public async Task<FileMeta> UploadFileAsync(
        Guid repoId, string path, byte[] content, long? ttlSeconds = null, CancellationToken ct = default)
    {
        using var stream = new MemoryStream(content);
        return await UploadFileAsync(repoId, path, stream, ttlSeconds, ct).ConfigureAwait(false);
    }

    public async Task<FileMeta> UploadFileAsync(
        Guid repoId, string path, Stream content, long? ttlSeconds = null, CancellationToken ct = default)
    {
        var encodedPath = EncodePath(path);
        using var request = new HttpRequestMessage(HttpMethod.Post, $"/api/v1/repos/{repoId}/files/{encodedPath}");

        var streamContent = new StreamContent(content);
        streamContent.Headers.ContentType = new MediaTypeHeaderValue("application/octet-stream");
        request.Content = streamContent;

        if (ttlSeconds.HasValue)
        {
            request.Headers.TryAddWithoutValidation("X-File-TTL", ttlSeconds.Value.ToString());
        }

        using var response = await _httpClient.SendAsync(request, ct).ConfigureAwait(false);
        await ThrowOnErrorAsync(response, ct).ConfigureAwait(false);

        var apiResponse = await DeserializeAsync<ApiResponse<FileMeta>>(response, ct).ConfigureAwait(false);
        return apiResponse?.Data ?? throw new LinuxFsException("Failed to deserialize upload response.");
    }

    public async Task<Stream> DownloadFileAsync(
        Guid repoId, string path, string? ifNoneMatch = null, CancellationToken ct = default)
    {
        var encodedPath = EncodePath(path);
        using var request = new HttpRequestMessage(HttpMethod.Get, $"/api/v1/repos/{repoId}/files/{encodedPath}");

        if (!string.IsNullOrEmpty(ifNoneMatch))
        {
            request.Headers.TryAddWithoutValidation("If-None-Match", ifNoneMatch);
        }

        var response = await _httpClient.SendAsync(request, HttpCompletionOption.ResponseHeadersRead, ct)
            .ConfigureAwait(false);

        if (response.StatusCode == HttpStatusCode.NotModified)
        {
            response.Dispose();
            return Stream.Null;
        }

        await ThrowOnErrorAsync(response, ct).ConfigureAwait(false);

        return await response.Content.ReadAsStreamAsync(ct).ConfigureAwait(false);
    }

    public async Task<FileHeadResponse> HeadFileAsync(Guid repoId, string path, CancellationToken ct = default)
    {
        var encodedPath = EncodePath(path);
        using var request = new HttpRequestMessage(HttpMethod.Head, $"/api/v1/repos/{repoId}/files/{encodedPath}");

        using var response = await _httpClient.SendAsync(request, ct).ConfigureAwait(false);
        await ThrowOnErrorAsync(response, ct).ConfigureAwait(false);

        var result = new FileHeadResponse
        {
            ContentType = response.Content.Headers.ContentType?.MediaType ?? string.Empty,
            ContentLength = response.Content.Headers.ContentLength ?? 0,
        };

        if (response.Headers.ETag is not null)
        {
            result.Etag = response.Headers.ETag.Tag;
        }

        if (response.Content.Headers.LastModified.HasValue)
        {
            result.LastModified = response.Content.Headers.LastModified.Value;
        }

        return result;
    }

    public async Task DeleteFileAsync(Guid repoId, string path, CancellationToken ct = default)
    {
        var encodedPath = EncodePath(path);
        await SendNoContentAsync(HttpMethod.Delete, $"/api/v1/repos/{repoId}/files/{encodedPath}", ct: ct)
            .ConfigureAwait(false);
    }

    public async Task<FileMeta> MoveFileAsync(Guid repoId, MoveFileRequest request, CancellationToken ct = default)
    {
        return await SendAndDeserializeAsync<FileMeta>(
            HttpMethod.Post,
            $"/api/v1/repos/{repoId}/files-move",
            JsonContent(request),
            ct
        ).ConfigureAwait(false);
    }

    public async Task<FileMeta> CopyFileAsync(Guid repoId, CopyFileRequest request, CancellationToken ct = default)
    {
        return await SendAndDeserializeAsync<FileMeta>(
            HttpMethod.Post,
            $"/api/v1/repos/{repoId}/files-copy",
            JsonContent(request),
            ct
        ).ConfigureAwait(false);
    }

    // ──────────────────── Shell ────────────────────

    public async Task<ExecResponse> ExecAsync(Guid repoId, ExecRequest request, CancellationToken ct = default)
    {
        return await SendAndDeserializeAsync<ExecResponse>(
            HttpMethod.Post,
            $"/api/v1/repos/{repoId}/exec",
            JsonContent(request),
            ct
        ).ConfigureAwait(false);
    }

    // ──────────────────── Archive ────────────────────

    public async Task<Stream> CreateArchiveAsync(Guid repoId, ArchiveRequest request, CancellationToken ct = default)
    {
        using var httpRequest = new HttpRequestMessage(HttpMethod.Post, $"/api/v1/repos/{repoId}/archive");
        httpRequest.Content = JsonContent(request);

        var response = await _httpClient.SendAsync(httpRequest, HttpCompletionOption.ResponseHeadersRead, ct)
            .ConfigureAwait(false);

        await ThrowOnErrorAsync(response, ct).ConfigureAwait(false);

        return await response.Content.ReadAsStreamAsync(ct).ConfigureAwait(false);
    }

    // ──────────────────── Private Helpers ────────────────────

    /// <summary>
    /// Sends a request, unwraps the ApiResponse envelope, and returns the data payload.
    /// Throws typed exceptions on error responses.
    /// </summary>
    private async Task<T> SendAndDeserializeAsync<T>(
        HttpMethod method,
        string requestUri,
        HttpContent? content = null,
        CancellationToken ct = default)
    {
        using var request = new HttpRequestMessage(method, requestUri);
        request.Content = content;

        using var response = await _httpClient.SendAsync(request, ct).ConfigureAwait(false);
        await ThrowOnErrorAsync(response, ct).ConfigureAwait(false);

        var apiResponse = await DeserializeAsync<ApiResponse<T>>(response, ct).ConfigureAwait(false);

        if (apiResponse is null)
        {
            throw new LinuxFsException($"Failed to deserialize response from {method} {requestUri}.");
        }

        if (apiResponse.Error is not null)
        {
            throw CreateApiException((HttpStatusCode)apiResponse.Error.Code, apiResponse.Error.Code, apiResponse.Error.Message);
        }

        return apiResponse.Data!;
    }

    /// <summary>
    /// Sends a request and expects a 204 No Content response.
    /// </summary>
    private async Task SendNoContentAsync(
        HttpMethod method,
        string requestUri,
        HttpContent? content = null,
        CancellationToken ct = default)
    {
        using var request = new HttpRequestMessage(method, requestUri);
        request.Content = content;

        using var response = await _httpClient.SendAsync(request, ct).ConfigureAwait(false);
        await ThrowOnErrorAsync(response, ct).ConfigureAwait(false);
    }

    /// <summary>
    /// Checks the HTTP response for error status codes and throws typed exceptions.
    /// </summary>
    private static async Task ThrowOnErrorAsync(HttpResponseMessage response, CancellationToken ct)
    {
        if (response.IsSuccessStatusCode)
        {
            return;
        }

        string body;
        try
        {
            body = await response.Content.ReadAsStringAsync(ct).ConfigureAwait(false);
        }
        catch
        {
            throw CreateApiException(response.StatusCode, (int)response.StatusCode, response.ReasonPhrase ?? "Unknown error");
        }

        try
        {
            var errorResponse = JsonSerializer.Deserialize<ApiResponse<object>>(body, JsonOptions);
            if (errorResponse?.Error is not null)
            {
                throw CreateApiException(response.StatusCode, errorResponse.Error.Code, errorResponse.Error.Message);
            }
        }
        catch (LinuxFsApiException)
        {
            throw;
        }
        catch
        {
            // Could not parse error JSON, fall through to generic exception
        }

        throw CreateApiException(response.StatusCode, (int)response.StatusCode, body);
    }

    /// <summary>
    /// Creates the appropriate typed exception based on the HTTP status code.
    /// </summary>
    private static LinuxFsApiException CreateApiException(HttpStatusCode statusCode, int errorCode, string message)
    {
        return statusCode switch
        {
            HttpStatusCode.NotFound => new LinuxFsNotFoundException(errorCode, message),
            HttpStatusCode.Conflict => new LinuxFsConflictException(errorCode, message),
            _ => new LinuxFsApiException(statusCode, errorCode, message),
        };
    }

    private static async Task<T?> DeserializeAsync<T>(HttpResponseMessage response, CancellationToken ct)
    {
        var stream = await response.Content.ReadAsStreamAsync(ct).ConfigureAwait(false);
        return await JsonSerializer.DeserializeAsync<T>(stream, JsonOptions, ct).ConfigureAwait(false);
    }

    private static StringContent JsonContent<T>(T obj)
    {
        var json = JsonSerializer.Serialize(obj, JsonOptions);
        return new StringContent(json, Encoding.UTF8, "application/json");
    }

    private static string BuildQuery(params (string key, string? value)[] parameters)
    {
        var queryParams = parameters
            .Where(p => p.value is not null)
            .Select(p => $"{HttpUtility.UrlEncode(p.key)}={HttpUtility.UrlEncode(p.value)}")
            .ToList();

        return queryParams.Count > 0 ? "?" + string.Join("&", queryParams) : string.Empty;
    }

    /// <summary>
    /// Encodes a file path for use in a URL, preserving forward slashes.
    /// </summary>
    private static string EncodePath(string path)
    {
        // Trim leading slash to avoid double-slash in URL
        var trimmed = path.TrimStart('/');
        var segments = trimmed.Split('/');
        return string.Join("/", segments.Select(Uri.EscapeDataString));
    }
}
