using FluentAssertions;
using LinuxFs.Client.Models;
using LinuxFs.Client.Tests.Helpers;
using Xunit;

namespace LinuxFs.Client.Tests;

public class HealthClientTests
{
    private static LinuxFsClient CreateClient(MockHttpHandler handler)
    {
        var httpClient = new HttpClient(handler) { BaseAddress = new Uri("http://localhost") };
        return new LinuxFsClient(httpClient);
    }

    [Fact]
    public async Task HealthCheckAsync_ReturnsOkStatus()
    {
        // Arrange
        var healthResponse = new HealthResponse { Status = "ok" };
        var handler = MockHttpHandler.WithJson(healthResponse);
        var client = CreateClient(handler);

        // Act
        var result = await client.HealthCheckAsync();

        // Assert
        result.Should().NotBeNull();
        result.Status.Should().Be("ok");

        handler.LastRequest!.Method.Should().Be(HttpMethod.Get);
        handler.LastRequest!.RequestUri!.PathAndQuery.Should().Be("/health");
    }

    [Fact]
    public async Task GetStatusAsync_DeserializesAllFields()
    {
        // Arrange
        var statusResponse = new StatusResponse
        {
            RepoCount = 5,
            TotalSizeBytes = 1_073_741_824,
            UptimeSeconds = 86400,
            Version = "0.1.0",
        };

        var handler = MockHttpHandler.WithApiResponse(statusResponse);
        var client = CreateClient(handler);

        // Act
        var result = await client.GetStatusAsync();

        // Assert
        result.Should().NotBeNull();
        result.RepoCount.Should().Be(5);
        result.TotalSizeBytes.Should().Be(1_073_741_824);
        result.UptimeSeconds.Should().Be(86400);
        result.Version.Should().Be("0.1.0");

        handler.LastRequest!.Method.Should().Be(HttpMethod.Get);
        handler.LastRequest!.RequestUri!.PathAndQuery.Should().Be("/api/v1/status");
    }

    [Fact]
    public async Task GetStatusAsync_HandlesZeroValues()
    {
        // Arrange
        var statusResponse = new StatusResponse
        {
            RepoCount = 0,
            TotalSizeBytes = 0,
            UptimeSeconds = 0,
            Version = "0.1.0",
        };

        var handler = MockHttpHandler.WithApiResponse(statusResponse);
        var client = CreateClient(handler);

        // Act
        var result = await client.GetStatusAsync();

        // Assert
        result.RepoCount.Should().Be(0);
        result.TotalSizeBytes.Should().Be(0);
        result.UptimeSeconds.Should().Be(0);
    }
}
