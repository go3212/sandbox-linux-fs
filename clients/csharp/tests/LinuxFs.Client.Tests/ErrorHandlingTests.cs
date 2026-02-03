using System.Net;
using FluentAssertions;
using LinuxFs.Client.Exceptions;
using LinuxFs.Client.Tests.Helpers;
using Xunit;

namespace LinuxFs.Client.Tests;

public class ErrorHandlingTests
{
    private static LinuxFsClient CreateClient(MockHttpHandler handler)
    {
        var httpClient = new HttpClient(handler) { BaseAddress = new Uri("http://localhost") };
        return new LinuxFsClient(httpClient);
    }

    [Fact]
    public async Task ApiReturns404_ThrowsLinuxFsNotFoundException()
    {
        // Arrange
        var handler = MockHttpHandler.WithError(404, "Not found");
        var client = CreateClient(handler);

        // Act
        var act = () => client.GetRepoAsync(Guid.NewGuid());

        // Assert
        var ex = await act.Should().ThrowAsync<LinuxFsNotFoundException>();
        ex.Which.StatusCode.Should().Be(HttpStatusCode.NotFound);
        ex.Which.ErrorCode.Should().Be(404);
        ex.Which.Message.Should().Be("Not found");
    }

    [Fact]
    public async Task ApiReturns409_ThrowsLinuxFsConflictException()
    {
        // Arrange
        var handler = MockHttpHandler.WithError(409, "Resource conflict");
        var client = CreateClient(handler);

        // Act
        var act = () => client.CreateRepoAsync(new Models.CreateRepoRequest { Name = "dup" });

        // Assert
        var ex = await act.Should().ThrowAsync<LinuxFsConflictException>();
        ex.Which.StatusCode.Should().Be(HttpStatusCode.Conflict);
        ex.Which.ErrorCode.Should().Be(409);
        ex.Which.Message.Should().Be("Resource conflict");
    }

    [Fact]
    public async Task ApiReturns401_ThrowsLinuxFsApiException()
    {
        // Arrange
        var handler = MockHttpHandler.WithError(401, "Unauthorized");
        var client = CreateClient(handler);

        // Act
        var act = () => client.GetStatusAsync();

        // Assert
        var ex = await act.Should().ThrowAsync<LinuxFsApiException>();
        ex.Which.StatusCode.Should().Be(HttpStatusCode.Unauthorized);
        ex.Which.ErrorCode.Should().Be(401);
        ex.Which.Message.Should().Be("Unauthorized");
    }

    [Fact]
    public async Task ApiReturns500_ThrowsLinuxFsApiException()
    {
        // Arrange
        var handler = MockHttpHandler.WithError(500, "Internal server error");
        var client = CreateClient(handler);

        // Act
        var act = () => client.GetStatusAsync();

        // Assert
        var ex = await act.Should().ThrowAsync<LinuxFsApiException>();
        ex.Which.StatusCode.Should().Be(HttpStatusCode.InternalServerError);
        ex.Which.ErrorCode.Should().Be(500);
        ex.Which.Message.Should().Be("Internal server error");
    }

    [Fact]
    public async Task NotFoundIsSubclassOfApiException()
    {
        // Arrange
        var handler = MockHttpHandler.WithError(404, "Missing");
        var client = CreateClient(handler);

        // Act
        var act = () => client.GetRepoAsync(Guid.NewGuid());

        // Assert
        // Should be catchable as LinuxFsApiException
        await act.Should().ThrowAsync<LinuxFsApiException>();
    }

    [Fact]
    public async Task ConflictIsSubclassOfApiException()
    {
        // Arrange
        var handler = MockHttpHandler.WithError(409, "Conflict");
        var client = CreateClient(handler);

        // Act
        var act = () => client.CreateRepoAsync(new Models.CreateRepoRequest { Name = "x" });

        // Assert
        await act.Should().ThrowAsync<LinuxFsApiException>();
    }

    [Fact]
    public async Task AllExceptionsInheritFromLinuxFsException()
    {
        // Arrange
        var handler = MockHttpHandler.WithError(404, "Not found");
        var client = CreateClient(handler);

        // Act
        var act = () => client.GetRepoAsync(Guid.NewGuid());

        // Assert
        await act.Should().ThrowAsync<LinuxFsException>();
    }

    [Fact]
    public async Task DeleteFile_404_ThrowsNotFoundException()
    {
        // Arrange
        var handler = MockHttpHandler.WithError(404, "File not found");
        var client = CreateClient(handler);

        // Act
        var act = () => client.DeleteFileAsync(Guid.NewGuid(), "nonexistent.txt");

        // Assert
        var ex = await act.Should().ThrowAsync<LinuxFsNotFoundException>();
        ex.Which.Message.Should().Be("File not found");
    }
}
