using System.Text.Json;
using System.Text.Json.Serialization;
using FluentAssertions;
using LinuxFs.Client.Models;
using Xunit;

namespace LinuxFs.Client.Tests;

public class SerializationTests
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    [Fact]
    public void PascalCaseProperties_SerializeAsSnakeCase()
    {
        // Arrange
        var repo = new RepoMeta
        {
            Id = Guid.Parse("11111111-1111-1111-1111-111111111111"),
            Name = "test",
            MaxSizeBytes = 1024,
            CurrentSizeBytes = 512,
            FileCount = 3,
            CreatedAt = DateTimeOffset.UtcNow,
            UpdatedAt = DateTimeOffset.UtcNow,
            LastAccessedAt = DateTimeOffset.UtcNow,
            DefaultTtlSeconds = 3600,
            Tags = new Dictionary<string, string>(),
        };

        // Act
        var json = JsonSerializer.Serialize(repo, JsonOptions);
        var doc = JsonDocument.Parse(json);

        // Assert
        doc.RootElement.TryGetProperty("max_size_bytes", out _).Should().BeTrue();
        doc.RootElement.TryGetProperty("current_size_bytes", out _).Should().BeTrue();
        doc.RootElement.TryGetProperty("file_count", out _).Should().BeTrue();
        doc.RootElement.TryGetProperty("created_at", out _).Should().BeTrue();
        doc.RootElement.TryGetProperty("updated_at", out _).Should().BeTrue();
        doc.RootElement.TryGetProperty("last_accessed_at", out _).Should().BeTrue();
        doc.RootElement.TryGetProperty("default_ttl_seconds", out _).Should().BeTrue();

        // PascalCase keys should NOT be present
        doc.RootElement.TryGetProperty("MaxSizeBytes", out _).Should().BeFalse();
        doc.RootElement.TryGetProperty("CurrentSizeBytes", out _).Should().BeFalse();
        doc.RootElement.TryGetProperty("FileCount", out _).Should().BeFalse();
    }

    [Fact]
    public void NullOptionalFields_AreOmittedFromJson()
    {
        // Arrange
        var request = new CreateRepoRequest
        {
            Name = "my-repo",
            // MaxSizeBytes is null
            // DefaultTtlSeconds is null
        };

        // Act
        var json = JsonSerializer.Serialize(request, JsonOptions);
        var doc = JsonDocument.Parse(json);

        // Assert
        doc.RootElement.TryGetProperty("name", out _).Should().BeTrue();
        doc.RootElement.TryGetProperty("max_size_bytes", out _).Should().BeFalse();
        doc.RootElement.TryGetProperty("default_ttl_seconds", out _).Should().BeFalse();
    }

    [Fact]
    public void UpdateRepoRequest_NullFieldsAreOmitted()
    {
        // Arrange
        var request = new UpdateRepoRequest
        {
            Name = "updated",
            // All other fields null
        };

        // Act
        var json = JsonSerializer.Serialize(request, JsonOptions);
        var doc = JsonDocument.Parse(json);

        // Assert
        doc.RootElement.TryGetProperty("name", out var nameEl).Should().BeTrue();
        nameEl.GetString().Should().Be("updated");
        doc.RootElement.TryGetProperty("max_size_bytes", out _).Should().BeFalse();
        doc.RootElement.TryGetProperty("default_ttl_seconds", out _).Should().BeFalse();
        doc.RootElement.TryGetProperty("tags", out _).Should().BeFalse();
    }

    [Fact]
    public void ExecRequest_OptionalFieldsOmittedWhenNull()
    {
        // Arrange
        var request = new ExecRequest
        {
            Command = "ls",
            Args = new List<string> { "-la" },
        };

        // Act
        var json = JsonSerializer.Serialize(request, JsonOptions);
        var doc = JsonDocument.Parse(json);

        // Assert
        doc.RootElement.GetProperty("command").GetString().Should().Be("ls");
        doc.RootElement.GetProperty("args").GetArrayLength().Should().Be(1);
        doc.RootElement.TryGetProperty("timeout_seconds", out _).Should().BeFalse();
        doc.RootElement.TryGetProperty("max_output_bytes", out _).Should().BeFalse();
    }

    [Fact]
    public void ArchiveRequest_OptionalFieldsOmittedWhenNull()
    {
        // Arrange
        var request = new ArchiveRequest();

        // Act
        var json = JsonSerializer.Serialize(request, JsonOptions);
        var doc = JsonDocument.Parse(json);

        // Assert
        doc.RootElement.TryGetProperty("path", out _).Should().BeFalse();
        doc.RootElement.TryGetProperty("format", out _).Should().BeFalse();
    }

    [Fact]
    public void DateTimeOffset_RoundTripsCorrectly()
    {
        // Arrange
        var now = new DateTimeOffset(2025, 6, 15, 12, 30, 45, TimeSpan.Zero);
        var fileMeta = new FileMeta
        {
            RepoId = Guid.NewGuid(),
            Path = "test.txt",
            SizeBytes = 100,
            Etag = "\"abc\"",
            ContentType = "text/plain",
            CreatedAt = now,
            UpdatedAt = now,
            LastAccessedAt = now,
            AccessCount = 1,
            ExpiresAt = now.AddHours(1),
        };

        // Act
        var json = JsonSerializer.Serialize(fileMeta, JsonOptions);
        var deserialized = JsonSerializer.Deserialize<FileMeta>(json, JsonOptions);

        // Assert
        deserialized.Should().NotBeNull();
        deserialized!.CreatedAt.Should().Be(now);
        deserialized.UpdatedAt.Should().Be(now);
        deserialized.LastAccessedAt.Should().Be(now);
        deserialized.ExpiresAt.Should().Be(now.AddHours(1));
    }

    [Fact]
    public void RepoMeta_NullDefaultTtl_OmittedInSerialization()
    {
        // Arrange
        var repo = new RepoMeta
        {
            Id = Guid.NewGuid(),
            Name = "test",
            DefaultTtlSeconds = null,
        };

        // Act
        var json = JsonSerializer.Serialize(repo, JsonOptions);
        var doc = JsonDocument.Parse(json);

        // Assert
        doc.RootElement.TryGetProperty("default_ttl_seconds", out _).Should().BeFalse();
    }

    [Fact]
    public void FileMeta_NullExpiresAt_OmittedInSerialization()
    {
        // Arrange
        var file = new FileMeta
        {
            RepoId = Guid.NewGuid(),
            Path = "test.txt",
            ExpiresAt = null,
        };

        // Act
        var json = JsonSerializer.Serialize(file, JsonOptions);
        var doc = JsonDocument.Parse(json);

        // Assert
        doc.RootElement.TryGetProperty("expires_at", out _).Should().BeFalse();
    }

    [Fact]
    public void ApiResponse_DeserializesWithSnakeCaseFields()
    {
        // Arrange
        var json = """
        {
            "data": {
                "repo_count": 10,
                "total_size_bytes": 999999,
                "uptime_seconds": 3600,
                "version": "0.1.0"
            },
            "error": null
        }
        """;

        // Act
        var result = JsonSerializer.Deserialize<ApiResponse<StatusResponse>>(json, JsonOptions);

        // Assert
        result.Should().NotBeNull();
        result!.Data.Should().NotBeNull();
        result.Data!.RepoCount.Should().Be(10);
        result.Data.TotalSizeBytes.Should().Be(999999);
        result.Data.UptimeSeconds.Should().Be(3600);
        result.Data.Version.Should().Be("0.1.0");
        result.Error.Should().BeNull();
    }

    [Fact]
    public void ApiResponse_DeserializesErrorCorrectly()
    {
        // Arrange
        var json = """
        {
            "data": null,
            "error": {
                "code": 404,
                "message": "Not found"
            }
        }
        """;

        // Act
        var result = JsonSerializer.Deserialize<ApiResponse<object>>(json, JsonOptions);

        // Assert
        result.Should().NotBeNull();
        result!.Data.Should().BeNull();
        result.Error.Should().NotBeNull();
        result.Error!.Code.Should().Be(404);
        result.Error.Message.Should().Be("Not found");
    }

    [Fact]
    public void MoveFileRequest_SerializesCorrectly()
    {
        // Arrange
        var request = new MoveFileRequest
        {
            Source = "a/b.txt",
            Destination = "c/d.txt",
        };

        // Act
        var json = JsonSerializer.Serialize(request, JsonOptions);
        var doc = JsonDocument.Parse(json);

        // Assert
        doc.RootElement.GetProperty("source").GetString().Should().Be("a/b.txt");
        doc.RootElement.GetProperty("destination").GetString().Should().Be("c/d.txt");
    }
}
