package com.rwid.config;

import com.rwid.security.JwtAuthenticationFilter;
import com.rwid.security.JwtTokenProvider;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.http.HttpMethod;
import org.springframework.security.authentication.AuthenticationManager;
import org.springframework.security.config.annotation.authentication.configuration.AuthenticationConfiguration;
import org.springframework.security.config.annotation.method.configuration.EnableMethodSecurity;
import org.springframework.security.config.annotation.web.builders.HttpSecurity;
import org.springframework.security.config.annotation.web.configuration.EnableWebSecurity;
import org.springframework.security.config.http.SessionCreationPolicy;
import org.springframework.security.web.SecurityFilterChain;
import org.springframework.security.web.authentication.UsernamePasswordAuthenticationFilter;
import org.springframework.web.cors.CorsConfiguration;
import org.springframework.web.cors.CorsConfigurationSource;
import org.springframework.web.cors.UrlBasedCorsConfigurationSource;

import java.util.Arrays;
import java.util.List;

@Configuration
@EnableWebSecurity
@EnableMethodSecurity
public class SecurityConfig {

    private final JwtTokenProvider jwtTokenProvider;

    @Value("${CORS_ALLOWED_ORIGINS}")
    private String allowedOrigins;

    public SecurityConfig(JwtTokenProvider jwtTokenProvider) {
        this.jwtTokenProvider = jwtTokenProvider;
    }

    @Bean
    public AuthenticationManager authenticationManager(AuthenticationConfiguration config) throws Exception {
        return config.getAuthenticationManager();
    }

    @Bean
    public CorsConfigurationSource corsConfigurationSource() {
        CorsConfiguration configuration = new CorsConfiguration();
        List<String> origins = Arrays.asList(allowedOrigins.split(","));
        configuration.setAllowedOrigins(origins);
        configuration.setAllowedMethods(Arrays.asList("GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"));
        configuration.setAllowedHeaders(Arrays.asList("*"));
        configuration.setExposedHeaders(Arrays.asList("Content-Type", "Authorization", "X-Total-Count"));
        configuration.setAllowCredentials(true);
        configuration.setMaxAge(3600L);

        UrlBasedCorsConfigurationSource source = new UrlBasedCorsConfigurationSource();
        source.registerCorsConfiguration("/**", configuration);
        return source;
    }

    @Bean
    public SecurityFilterChain filterChain(HttpSecurity http) throws Exception {
        http
            .csrf().disable()
            .cors(cors -> cors.configurationSource(corsConfigurationSource()))
            .sessionManagement(session -> session.sessionCreationPolicy(SessionCreationPolicy.STATELESS))
            .exceptionHandling(exception -> exception
                .authenticationEntryPoint((request, response, authException) -> {
                    response.setStatus(jakarta.servlet.http.HttpServletResponse.SC_UNAUTHORIZED);
                    response.setContentType("application/json");
                    response.getWriter().write("{\"error\": \"Unauthorized\", \"message\": \"Unauthorized: " + authException.getMessage() + "\"}");
                })
            )
            .authorizeHttpRequests(authz -> authz
                .requestMatchers(HttpMethod.OPTIONS, "/**").permitAll()
                .requestMatchers(HttpMethod.POST, "/auth/login", "/auth/register", "/auth/register-with-profile", "/auth/oauth/google/callback", "/auth/users/check-existence").permitAll()
                .requestMatchers(HttpMethod.PUT, "/auth/change-password").permitAll()
                .requestMatchers(HttpMethod.GET, "/auth/oauth/google").permitAll()
                .requestMatchers(HttpMethod.GET, "/schools").permitAll()
                .requestMatchers(HttpMethod.POST, "/schools").permitAll()
                .requestMatchers(HttpMethod.GET, "/users").permitAll()
                .requestMatchers(HttpMethod.GET, "/platforms").permitAll()
                .requestMatchers(HttpMethod.GET, "/platforms/slug/**").permitAll()
                .requestMatchers(HttpMethod.GET, "/posts/**").permitAll()
                .requestMatchers(HttpMethod.GET, "/app-config").permitAll()
                .requestMatchers(HttpMethod.PUT, "/app-config").authenticated()
                .requestMatchers(HttpMethod.GET, "/homepage-layout/published").permitAll()
                .requestMatchers(HttpMethod.GET, "/files/**").permitAll()
                .requestMatchers(HttpMethod.GET, "/users/**").permitAll()
                .requestMatchers(HttpMethod.GET, "/events/homepage").permitAll()
                .requestMatchers(HttpMethod.GET, "/events/**").permitAll()
                .requestMatchers(HttpMethod.POST, "/events").authenticated()
                .requestMatchers(HttpMethod.PUT, "/events/**").authenticated()
                .requestMatchers(HttpMethod.DELETE, "/events/**").authenticated()
                .requestMatchers(HttpMethod.GET, "/communities").permitAll()
                .requestMatchers(HttpMethod.GET, "/communities/**").permitAll()
                .requestMatchers(HttpMethod.POST, "/communities").authenticated()
                .requestMatchers(HttpMethod.PUT, "/communities/**").authenticated()
                .requestMatchers(HttpMethod.DELETE, "/communities/**").authenticated()
                .requestMatchers(HttpMethod.GET, "/courses").permitAll()
                .requestMatchers(HttpMethod.GET, "/courses/**").permitAll()
                .requestMatchers(HttpMethod.POST, "/courses").authenticated()
                .requestMatchers(HttpMethod.PUT, "/courses/**").authenticated()
                .requestMatchers(HttpMethod.DELETE, "/courses/**").authenticated()
                .requestMatchers(HttpMethod.GET, "/notifications/unread").authenticated()
                .requestMatchers(HttpMethod.GET, "/notifications/unread-count").authenticated()
                .requestMatchers(HttpMethod.GET, "/slides/**").permitAll()
                .requestMatchers(HttpMethod.POST, "/slides").authenticated()
                .requestMatchers(HttpMethod.PUT, "/slides/**").authenticated()
                .requestMatchers(HttpMethod.DELETE, "/slides/**").authenticated()
                .requestMatchers("/setup/**").permitAll()
                .requestMatchers("/health", "/actuator/**").permitAll()
                .anyRequest().authenticated()
            )
            .addFilterBefore(new JwtAuthenticationFilter(jwtTokenProvider), UsernamePasswordAuthenticationFilter.class);

        return http.build();
    }
}
