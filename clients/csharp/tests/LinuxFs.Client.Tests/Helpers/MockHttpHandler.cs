using System.Net;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using LinuxFs.Client.Models;

namespace LinuxFs.Client.Tests.Helpers;

/// <summary>
/// A custom <see cref="HttpMessageHandler"/> that returns pre-configured responses for testing.
/// Also captures the last request for assertions.
/// </summary>
public sealed class MockHttpHandler : HttpMessageHandler
{
    private readonly Func<HttpRequestMessage, HttpResponseMessage> _responseFactory;

    /// <summary>
    /// The last request sent through this handler.
    /// </summary>
    public HttpRequestMessage? LastRequest { get; private set; }

    /// <summary>
    /// The body string of the last request, captured before the content is disposed.
    /// </summary>
    public string? LastRequestBody { get; private set; }

    private MockHttpHandler(Func<HttpRequestMessage, HttpResponseMessage> responseFactory)
    {
        _responseFactory = responseFactory;
    }

    protected override async Task<HttpResponseMessage> SendAsync(
        HttpRequestMessage request,
        CancellationToken cancellationToken)
    {
        LastRequest = request;

        if (request.Content is not null)
        {
            LastRequestBody = await request.Content.ReadAsStringAsync(cancellationToken)
                .ConfigureAwait(false);
        }
        else
        {
            LastRequestBody = null;
        }

        return _responseFactory(request);
    }

    // ──────────────────── Factory Methods ────────────────────

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull,
    };

    /// <summary>
    /// Creates a handler that returns a raw JSON response (not wrapped in ApiResponse).
    /// </summary>
    public static MockHttpHandler WithJson<T>(T data, HttpStatusCode statusCode = HttpStatusCode.OK)
    {
        return new MockHttpHandler(_ =>
        {
            var json = JsonSerializer.Serialize(data, JsonOptions);
            var response = new HttpResponseMessage(statusCode)
            {
                Content = new StringContent(json, Encoding.UTF8, "application/json"),
            };
            return response;
        });
    }

    /// <summary>
    /// Creates a handler that returns data wrapped in an <see cref="ApiResponse{T}"/> envelope.
    /// </summary>
    public static MockHttpHandler WithApiResponse<T>(T data, HttpStatusCode statusCode = HttpStatusCode.OK)
    {
        return new MockHttpHandler(_ =>
        {
            var wrapper = new ApiResponse<T> { Data = data, Error = null };
            var json = JsonSerializer.Serialize(wrapper, JsonOptions);
            var response = new HttpResponseMessage(statusCode)
            {
                Content = new StringContent(json, Encoding.UTF8, "application/json"),
            };
            return response;
        });
    }

    /// <summary>
    /// Creates a handler that returns an API error response.
    /// </summary>
    public static MockHttpHandler WithError(int code, string message)
    {
        return new MockHttpHandler(_ =>
        {
            var wrapper = new ApiResponse<object>
            {
                Data = null,
                Error = new ApiError { Code = code, Message = message },
            };
            var json = JsonSerializer.Serialize(wrapper, JsonOptions);
            var httpStatusCode = (HttpStatusCode)code;
            var response = new HttpResponseMessage(httpStatusCode)
            {
                Content = new StringContent(json, Encoding.UTF8, "application/json"),
            };
            return response;
        });
    }

    /// <summary>
    /// Creates a handler that returns a 204 No Content response.
    /// </summary>
    public static MockHttpHandler WithNoContent()
    {
        return new MockHttpHandler(_ => new HttpResponseMessage(HttpStatusCode.NoContent));
    }

    /// <summary>
    /// Creates a handler that returns binary content as a stream.
    /// </summary>
    public static MockHttpHandler WithStream(byte[] bytes, string contentType = "application/octet-stream")
    {
        return new MockHttpHandler(_ =>
        {
            var response = new HttpResponseMessage(HttpStatusCode.OK)
            {
                Content = new ByteArrayContent(bytes),
            };
            response.Content.Headers.ContentType = new MediaTypeHeaderValue(contentType);
            return response;
        });
    }

    /// <summary>
    /// Creates a handler that returns a HEAD response with specific headers.
    /// </summary>
    public static MockHttpHandler WithHeadResponse(
        string contentType,
        long contentLength,
        string? etag = null,
        DateTimeOffset? lastModified = null)
    {
        return new MockHttpHandler(_ =>
        {
            var response = new HttpResponseMessage(HttpStatusCode.OK)
            {
                Content = new ByteArrayContent(Array.Empty<byte>()),
            };
            response.Content.Headers.ContentType = new MediaTypeHeaderValue(contentType);
            response.Content.Headers.ContentLength = contentLength;

            if (etag is not null)
            {
                response.Headers.ETag = new EntityTagHeaderValue($"\"{etag}\"");
            }

            if (lastModified.HasValue)
            {
                response.Content.Headers.LastModified = lastModified.Value;
            }

            return response;
        });
    }
}
