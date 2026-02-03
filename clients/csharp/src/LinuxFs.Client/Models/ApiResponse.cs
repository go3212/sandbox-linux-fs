using System.Text.Json.Serialization;

namespace LinuxFs.Client.Models;

/// <summary>
/// Standard envelope for all API responses.
/// </summary>
/// <typeparam name="T">The type of the data payload.</typeparam>
public sealed class ApiResponse<T>
{
    [JsonPropertyName("data")]
    public T? Data { get; set; }

    [JsonPropertyName("error")]
    public ApiError? Error { get; set; }
}

/// <summary>
/// Error details returned by the API.
/// </summary>
public sealed class ApiError
{
    [JsonPropertyName("code")]
    public int Code { get; set; }

    [JsonPropertyName("message")]
    public string Message { get; set; } = string.Empty;
}
