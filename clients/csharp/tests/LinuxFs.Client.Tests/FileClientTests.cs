using System.Text;
using System.Text.Json;
using FluentAssertions;
using LinuxFs.Client.Models;
using LinuxFs.Client.Tests.Helpers;
using Xunit;

namespace LinuxFs.Client.Tests;

public class FileClientTests
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull,
    };

    private static LinuxFsClient CreateClient(MockHttpHandler handler)
    {
        var httpClient = new HttpClient(handler) { BaseAddress = new Uri("http://localhost") };
        return new LinuxFsClient(httpClient);
    }

    [Fact]
    public async Task UploadFileAsync_SendsBinaryContentAndTtlHeader()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var fileMeta = new FileMeta
        {
            RepoId = repoId,
            Path = "docs/readme.txt",
            SizeBytes = 13,
            Etag = "\"abc123\"",
            ContentType = "text/plain",
            CreatedAt = DateTimeOffset.UtcNow,
            UpdatedAt = DateTimeOffset.UtcNow,
            LastAccessedAt = DateTimeOffset.UtcNow,
            AccessCount = 0,
        };

        var handler = MockHttpHandler.WithApiResponse(fileMeta, System.Net.HttpStatusCode.Created);
        var client = CreateClient(handler);

        var content = Encoding.UTF8.GetBytes("Hello, World!");

        // Act
        var result = await client.UploadFileAsync(repoId, "docs/readme.txt", content, ttlSeconds: 7200);

        // Assert
        result.Should().NotBeNull();
        result.Path.Should().Be("docs/readme.txt");
        result.SizeBytes.Should().Be(13);
        result.Etag.Should().Be("\"abc123\"");

        handler.LastRequest!.Method.Should().Be(HttpMethod.Post);
        handler.LastRequest!.RequestUri!.PathAndQuery.Should().Contain($"/api/v1/repos/{repoId}/files/docs/readme.txt");
        handler.LastRequest!.Headers.GetValues("X-File-TTL").Should().ContainSingle().Which.Should().Be("7200");

        handler.LastRequestBody.Should().Be("Hello, World!");
    }

    [Fact]
    public async Task UploadFileAsync_WithStream_SendsContentCorrectly()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var fileMeta = new FileMeta
        {
            RepoId = repoId,
            Path = "data.bin",
            SizeBytes = 4,
            Etag = "\"def456\"",
            ContentType = "application/octet-stream",
            CreatedAt = DateTimeOffset.UtcNow,
            UpdatedAt = DateTimeOffset.UtcNow,
            LastAccessedAt = DateTimeOffset.UtcNow,
            AccessCount = 0,
        };

        var handler = MockHttpHandler.WithApiResponse(fileMeta, System.Net.HttpStatusCode.Created);
        var client = CreateClient(handler);

        using var stream = new MemoryStream(new byte[] { 0xDE, 0xAD, 0xBE, 0xEF });

        // Act
        var result = await client.UploadFileAsync(repoId, "data.bin", stream);

        // Assert
        result.Path.Should().Be("data.bin");
        result.SizeBytes.Should().Be(4);
        handler.LastRequest!.Content!.Headers.ContentType!.MediaType.Should().Be("application/octet-stream");
    }

    [Fact]
    public async Task DownloadFileAsync_ReturnsStreamWithExpectedBytes()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var expectedBytes = Encoding.UTF8.GetBytes("file content here");

        var handler = MockHttpHandler.WithStream(expectedBytes);
        var client = CreateClient(handler);

        // Act
        using var resultStream = await client.DownloadFileAsync(repoId, "some/file.txt");
        using var ms = new MemoryStream();
        await resultStream.CopyToAsync(ms);
        var actualBytes = ms.ToArray();

        // Assert
        actualBytes.Should().BeEquivalentTo(expectedBytes);
        handler.LastRequest!.Method.Should().Be(HttpMethod.Get);
        handler.LastRequest!.RequestUri!.PathAndQuery.Should().Contain($"/api/v1/repos/{repoId}/files/some/file.txt");
    }

    [Fact]
    public async Task HeadFileAsync_ParsesResponseHeadersIntoFileHeadResponse()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var lastMod = new DateTimeOffset(2025, 1, 15, 10, 30, 0, TimeSpan.Zero);

        var handler = MockHttpHandler.WithHeadResponse(
            contentType: "text/plain",
            contentLength: 1024,
            etag: "abc123",
            lastModified: lastMod
        );
        var client = CreateClient(handler);

        // Act
        var result = await client.HeadFileAsync(repoId, "readme.md");

        // Assert
        result.ContentType.Should().Be("text/plain");
        result.ContentLength.Should().Be(1024);
        result.Etag.Should().Contain("abc123");
        result.LastModified.Should().NotBeNull();

        handler.LastRequest!.Method.Should().Be(HttpMethod.Head);
    }

    [Fact]
    public async Task MoveFileAsync_SendsCorrectJsonBody()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var fileMeta = new FileMeta
        {
            RepoId = repoId,
            Path = "new/path.txt",
            SizeBytes = 100,
            Etag = "\"etag1\"",
            ContentType = "text/plain",
            CreatedAt = DateTimeOffset.UtcNow,
            UpdatedAt = DateTimeOffset.UtcNow,
            LastAccessedAt = DateTimeOffset.UtcNow,
            AccessCount = 1,
        };

        var handler = MockHttpHandler.WithApiResponse(fileMeta);
        var client = CreateClient(handler);

        var moveRequest = new MoveFileRequest
        {
            Source = "old/path.txt",
            Destination = "new/path.txt",
        };

        // Act
        var result = await client.MoveFileAsync(repoId, moveRequest);

        // Assert
        result.Path.Should().Be("new/path.txt");

        handler.LastRequest!.Method.Should().Be(HttpMethod.Post);
        handler.LastRequest!.RequestUri!.PathAndQuery.Should().Be($"/api/v1/repos/{repoId}/files-move");

        var sentBody = JsonDocument.Parse(handler.LastRequestBody!);
        sentBody.RootElement.GetProperty("source").GetString().Should().Be("old/path.txt");
        sentBody.RootElement.GetProperty("destination").GetString().Should().Be("new/path.txt");
    }

    [Fact]
    public async Task CopyFileAsync_SendsCorrectJsonBody()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var fileMeta = new FileMeta
        {
            RepoId = repoId,
            Path = "copy/destination.txt",
            SizeBytes = 200,
            Etag = "\"etag2\"",
            ContentType = "text/plain",
            CreatedAt = DateTimeOffset.UtcNow,
            UpdatedAt = DateTimeOffset.UtcNow,
            LastAccessedAt = DateTimeOffset.UtcNow,
            AccessCount = 0,
        };

        var handler = MockHttpHandler.WithApiResponse(fileMeta);
        var client = CreateClient(handler);

        var copyRequest = new CopyFileRequest
        {
            Source = "original/file.txt",
            Destination = "copy/destination.txt",
        };

        // Act
        var result = await client.CopyFileAsync(repoId, copyRequest);

        // Assert
        result.Path.Should().Be("copy/destination.txt");

        handler.LastRequest!.Method.Should().Be(HttpMethod.Post);
        handler.LastRequest!.RequestUri!.PathAndQuery.Should().Be($"/api/v1/repos/{repoId}/files-copy");

        var sentBody = JsonDocument.Parse(handler.LastRequestBody!);
        sentBody.RootElement.GetProperty("source").GetString().Should().Be("original/file.txt");
        sentBody.RootElement.GetProperty("destination").GetString().Should().Be("copy/destination.txt");
    }

    [Fact]
    public async Task ListFilesAsync_SendsQueryParams()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var listResponse = new ListFilesResponse
        {
            Files = new List<FileMeta>
            {
                new() { RepoId = repoId, Path = "data/file1.txt", SizeBytes = 50 },
                new() { RepoId = repoId, Path = "data/file2.txt", SizeBytes = 75 },
            },
            Page = 1,
            PerPage = 20,
        };

        var handler = MockHttpHandler.WithApiResponse(listResponse);
        var client = CreateClient(handler);

        // Act
        var result = await client.ListFilesAsync(repoId, prefix: "data/", recursive: true, page: 1, perPage: 20);

        // Assert
        result.Files.Should().HaveCount(2);
        result.Page.Should().Be(1);
        result.PerPage.Should().Be(20);

        var query = handler.LastRequest!.RequestUri!.Query;
        query.Should().Contain("prefix=data");
        query.Should().Contain("recursive=true");
        query.Should().Contain("page=1");
        query.Should().Contain("per_page=20");
    }

    [Fact]
    public async Task DeleteFileAsync_ReturnsWithoutError()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var handler = MockHttpHandler.WithNoContent();
        var client = CreateClient(handler);

        // Act
        var act = () => client.DeleteFileAsync(repoId, "old/file.txt");

        // Assert
        await act.Should().NotThrowAsync();
        handler.LastRequest!.Method.Should().Be(HttpMethod.Delete);
        handler.LastRequest!.RequestUri!.PathAndQuery.Should().Contain($"/api/v1/repos/{repoId}/files/old/file.txt");
    }
}
