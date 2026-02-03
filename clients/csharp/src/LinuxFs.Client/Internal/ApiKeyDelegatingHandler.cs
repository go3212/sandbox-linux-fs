using Microsoft.Extensions.Options;

namespace LinuxFs.Client.Internal;

/// <summary>
/// Delegating handler that adds the X-API-Key header to every outgoing request.
/// </summary>
internal sealed class ApiKeyDelegatingHandler : DelegatingHandler
{
    private readonly LinuxFsClientOptions _options;

    public ApiKeyDelegatingHandler(IOptions<LinuxFsClientOptions> options)
    {
        _options = options.Value;
    }

    protected override Task<HttpResponseMessage> SendAsync(
        HttpRequestMessage request,
        CancellationToken cancellationToken)
    {
        if (!string.IsNullOrEmpty(_options.ApiKey))
        {
            request.Headers.TryAddWithoutValidation("X-API-Key", _options.ApiKey);
        }

        return base.SendAsync(request, cancellationToken);
    }
}
