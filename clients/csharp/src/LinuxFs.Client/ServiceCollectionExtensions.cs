using LinuxFs.Client.Internal;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;

namespace LinuxFs.Client;

/// <summary>
/// Extension methods for configuring the LinuxFs client with dependency injection.
/// </summary>
public static class ServiceCollectionExtensions
{
    /// <summary>
    /// Registers <see cref="ILinuxFsClient"/> and its dependencies with the service collection.
    /// </summary>
    /// <param name="services">The service collection.</param>
    /// <param name="configure">Action to configure <see cref="LinuxFsClientOptions"/>.</param>
    /// <returns>The service collection for chaining.</returns>
    public static IServiceCollection AddLinuxFsClient(
        this IServiceCollection services,
        Action<LinuxFsClientOptions> configure)
    {
        services.Configure(configure);

        services.AddTransient<ApiKeyDelegatingHandler>();

        services.AddHttpClient<ILinuxFsClient, LinuxFsClient>((sp, client) =>
        {
            var options = sp.GetRequiredService<IOptions<LinuxFsClientOptions>>().Value;

            if (!string.IsNullOrEmpty(options.BaseUrl))
            {
                client.BaseAddress = new Uri(options.BaseUrl.TrimEnd('/'));
            }
        })
        .AddHttpMessageHandler<ApiKeyDelegatingHandler>();

        return services;
    }
}
