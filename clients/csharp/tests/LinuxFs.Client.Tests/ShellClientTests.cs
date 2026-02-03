using System.Text.Json;
using FluentAssertions;
using LinuxFs.Client.Exceptions;
using LinuxFs.Client.Models;
using LinuxFs.Client.Tests.Helpers;
using Xunit;

namespace LinuxFs.Client.Tests;

public class ShellClientTests
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
    public async Task ExecAsync_SendsCorrectRequestBody()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var execResponse = new ExecResponse
        {
            ExitCode = 0,
            Stdout = "hello world\n",
            Stderr = "",
            DurationMs = 42,
            Truncated = false,
        };

        var handler = MockHttpHandler.WithApiResponse(execResponse);
        var client = CreateClient(handler);

        var execRequest = new ExecRequest
        {
            Command = "echo",
            Args = new List<string> { "hello", "world" },
            TimeoutSeconds = 30,
            MaxOutputBytes = 1_048_576,
        };

        // Act
        var result = await client.ExecAsync(repoId, execRequest);

        // Assert
        handler.LastRequest!.Method.Should().Be(HttpMethod.Post);
        handler.LastRequest!.RequestUri!.PathAndQuery.Should().Be($"/api/v1/repos/{repoId}/exec");

        var sentBody = JsonDocument.Parse(handler.LastRequestBody!);
        sentBody.RootElement.GetProperty("command").GetString().Should().Be("echo");

        var args = sentBody.RootElement.GetProperty("args");
        args.GetArrayLength().Should().Be(2);
        args[0].GetString().Should().Be("hello");
        args[1].GetString().Should().Be("world");

        sentBody.RootElement.GetProperty("timeout_seconds").GetUInt64().Should().Be(30UL);
        sentBody.RootElement.GetProperty("max_output_bytes").GetUInt64().Should().Be(1_048_576UL);
    }

    [Fact]
    public async Task ExecAsync_DeserializesAllFieldsCorrectly()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var execResponse = new ExecResponse
        {
            ExitCode = 1,
            Stdout = "partial output",
            Stderr = "error: something failed",
            DurationMs = 5000,
            Truncated = true,
        };

        var handler = MockHttpHandler.WithApiResponse(execResponse);
        var client = CreateClient(handler);

        var execRequest = new ExecRequest
        {
            Command = "make",
            Args = new List<string> { "build" },
        };

        // Act
        var result = await client.ExecAsync(repoId, execRequest);

        // Assert
        result.ExitCode.Should().Be(1);
        result.Stdout.Should().Be("partial output");
        result.Stderr.Should().Be("error: something failed");
        result.DurationMs.Should().Be(5000);
        result.Truncated.Should().BeTrue();
    }

    [Fact]
    public async Task ExecAsync_OptionalFieldsOmittedWhenNull()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var execResponse = new ExecResponse
        {
            ExitCode = 0,
            Stdout = "",
            Stderr = "",
            DurationMs = 10,
            Truncated = false,
        };

        var handler = MockHttpHandler.WithApiResponse(execResponse);
        var client = CreateClient(handler);

        var execRequest = new ExecRequest
        {
            Command = "ls",
            Args = new List<string>(),
            // TimeoutSeconds and MaxOutputBytes intentionally null
        };

        // Act
        await client.ExecAsync(repoId, execRequest);

        // Assert
        var sentBody = JsonDocument.Parse(handler.LastRequestBody!);
        sentBody.RootElement.TryGetProperty("timeout_seconds", out _).Should().BeFalse();
        sentBody.RootElement.TryGetProperty("max_output_bytes", out _).Should().BeFalse();
    }

    [Fact]
    public async Task ExecAsync_403_ThrowsLinuxFsApiException()
    {
        // Arrange
        var repoId = Guid.NewGuid();
        var handler = MockHttpHandler.WithError(403, "Forbidden: invalid API key");
        var client = CreateClient(handler);

        var execRequest = new ExecRequest
        {
            Command = "rm",
            Args = new List<string> { "-rf", "/" },
        };

        // Act
        var act = () => client.ExecAsync(repoId, execRequest);

        // Assert
        var ex = await act.Should().ThrowAsync<LinuxFsApiException>();
        ex.Which.StatusCode.Should().Be(System.Net.HttpStatusCode.Forbidden);
        ex.Which.Message.Should().Contain("Forbidden");
    }
}
