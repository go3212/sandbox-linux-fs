using System.Text.Json;
using FluentAssertions;
using LinuxFs.Client.Exceptions;
using LinuxFs.Client.Models;
using LinuxFs.Client.Tests.Helpers;
using Xunit;

namespace LinuxFs.Client.Tests;

public class RepoClientTests
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
    public async Task CreateRepoAsync_SendsCorrectPostBodyAndDeserializesRepoMeta()
    {
        // Arrange
        var expectedRepo = new RepoMeta
        {
            Id = Guid.NewGuid(),
            Name = "test-repo",
            MaxSizeBytes = 1_000_000,
            CurrentSizeBytes = 0,
            FileCount = 0,
            CreatedAt = DateTimeOffset.UtcNow,
            UpdatedAt = DateTimeOffset.UtcNow,
            LastAccessedAt = DateTimeOffset.UtcNow,
            DefaultTtlSeconds = 3600,
            Tags = new Dictionary<string, string> { ["env"] = "test" },
        };

        var handler = MockHttpHandler.WithApiResponse(expectedRepo, System.Net.HttpStatusCode.Created);
        var client = CreateClient(handler);

        var createRequest = new CreateRepoRequest
        {
            Name = "test-repo",
            MaxSizeBytes = 1_000_000,
            DefaultTtlSeconds = 3600,
        };

        // Act
        var result = await client.CreateRepoAsync(createRequest);

        // Assert
        result.Should().NotBeNull();
        result.Id.Should().Be(expectedRepo.Id);
        result.Name.Should().Be("test-repo");
        result.MaxSizeBytes.Should().Be(1_000_000);
        result.DefaultTtlSeconds.Should().Be(3600);

        handler.LastRequest!.Method.Should().Be(HttpMethod.Post);
        handler.LastRequest!.RequestUri!.PathAndQuery.Should().Be("/api/v1/repos");
        handler.LastRequestBody.Should().NotBeNullOrEmpty();

        var sentBody = JsonSerializer.Deserialize<CreateRepoRequest>(handler.LastRequestBody!, JsonOptions);
        sentBody!.Name.Should().Be("test-repo");
        sentBody.MaxSizeBytes.Should().Be(1_000_000UL);
        sentBody.DefaultTtlSeconds.Should().Be(3600UL);
    }

    [Fact]
    public async Task ListReposAsync_SendsQueryParamsCorrectly()
    {
        // Arrange
        var listResponse = new ListReposResponse
        {
            Repos = new List<RepoMeta>
            {
                new() { Id = Guid.NewGuid(), Name = "repo-1" },
                new() { Id = Guid.NewGuid(), Name = "repo-2" },
            },
            Page = 2,
            PerPage = 10,
            Total = 25,
        };

        var handler = MockHttpHandler.WithApiResponse(listResponse);
        var client = CreateClient(handler);

        // Act
        var result = await client.ListReposAsync(page: 2, perPage: 10, sort: "name");

        // Assert
        result.Repos.Should().HaveCount(2);
        result.Page.Should().Be(2);
        result.PerPage.Should().Be(10);
        result.Total.Should().Be(25);

        var query = handler.LastRequest!.RequestUri!.Query;
        query.Should().Contain("page=2");
        query.Should().Contain("per_page=10");
        query.Should().Contain("sort=name");
    }

    [Fact]
    public async Task GetRepoAsync_DeserializesGetRepoResponse()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var getRepoResponse = new GetRepoResponse
        {
            Repo = new RepoMeta { Id = repoId, Name = "my-repo" },
            FileCount = 42,
        };

        var handler = MockHttpHandler.WithApiResponse(getRepoResponse);
        var client = CreateClient(handler);

        // Act
        var result = await client.GetRepoAsync(repoId);

        // Assert
        result.Repo.Id.Should().Be(repoId);
        result.Repo.Name.Should().Be("my-repo");
        result.FileCount.Should().Be(42);

        handler.LastRequest!.RequestUri!.PathAndQuery.Should().Be($"/api/v1/repos/{repoId}");
    }

    [Fact]
    public async Task UpdateRepoAsync_SendsPatchWithCorrectBody()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var updatedRepo = new RepoMeta
        {
            Id = repoId,
            Name = "updated-name",
            MaxSizeBytes = 2_000_000,
        };

        var handler = MockHttpHandler.WithApiResponse(updatedRepo);
        var client = CreateClient(handler);

        var updateRequest = new UpdateRepoRequest
        {
            Name = "updated-name",
            MaxSizeBytes = 2_000_000,
            Tags = new Dictionary<string, string> { ["tier"] = "premium" },
        };

        // Act
        var result = await client.UpdateRepoAsync(repoId, updateRequest);

        // Assert
        result.Name.Should().Be("updated-name");
        result.MaxSizeBytes.Should().Be(2_000_000);

        handler.LastRequest!.Method.Should().Be(HttpMethod.Patch);
        handler.LastRequest!.RequestUri!.PathAndQuery.Should().Be($"/api/v1/repos/{repoId}");

        var sentBody = JsonDocument.Parse(handler.LastRequestBody!);
        sentBody.RootElement.GetProperty("name").GetString().Should().Be("updated-name");
        sentBody.RootElement.GetProperty("tags").GetProperty("tier").GetString().Should().Be("premium");
    }

    [Fact]
    public async Task DeleteRepoAsync_ReturnsWithoutError()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var handler = MockHttpHandler.WithNoContent();
        var client = CreateClient(handler);

        // Act
        var act = () => client.DeleteRepoAsync(repoId);

        // Assert
        await act.Should().NotThrowAsync();
        handler.LastRequest!.Method.Should().Be(HttpMethod.Delete);
        handler.LastRequest!.RequestUri!.PathAndQuery.Should().Be($"/api/v1/repos/{repoId}");
    }

    [Fact]
    public async Task GetRepoAsync_404_ThrowsLinuxFsNotFoundException()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var handler = MockHttpHandler.WithError(404, "Repository not found");
        var client = CreateClient(handler);

        // Act
        var act = () => client.GetRepoAsync(repoId);

        // Assert
        var ex = await act.Should().ThrowAsync<LinuxFsNotFoundException>();
        ex.Which.StatusCode.Should().Be(System.Net.HttpStatusCode.NotFound);
        ex.Which.Message.Should().Contain("Repository not found");
    }
}
