package com.rwid.service;

import com.rwid.dto.AuthResponse;
import com.rwid.dto.SetupRequest;
import com.rwid.dto.SetupStatusResponse;
import com.rwid.dto.UserDTO;
import com.rwid.model.AppConfig;
import com.rwid.model.User;
import com.rwid.repository.AppConfigRepository;
import com.rwid.repository.UserRepository;
import com.rwid.security.JwtTokenProvider;
import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Service;

import java.time.LocalDateTime;

@Slf4j
@Service
public class SetupService {

    private final UserRepository userRepository;
    private final AppConfigRepository appConfigRepository;
    private final UserService userService;
    private final JwtTokenProvider jwtTokenProvider;

    @Value("${app.api-base-url:http://localhost:9090/api}")
    private String apiBaseUrl;

    public SetupService(UserRepository userRepository, AppConfigRepository appConfigRepository,
                       UserService userService, JwtTokenProvider jwtTokenProvider) {
        this.userRepository = userRepository;
        this.appConfigRepository = appConfigRepository;
        this.userService = userService;
        this.jwtTokenProvider = jwtTokenProvider;
    }

    public SetupStatusResponse getSetupStatus() {
        long userCount = userRepository.count();
        boolean initialized = userCount > 0;

        return SetupStatusResponse.builder()
                .initialized(initialized)
                .message(initialized ? "System already initialized" : "System needs setup")
                .build();
    }

    public AuthResponse completeSetup(SetupRequest request) {
        log.info("Starting setup completion in Auth service");

        // Keep logoUrl as relative path - frontend will resolve it
        String logoUrl = request.getLogoUrl();

        // Save or update app config in Auth
        AppConfig appConfig = appConfigRepository.findAll().stream().findFirst().orElse(null);
        if (appConfig == null) {
            appConfig = AppConfig.builder()
                    .appName(request.getPlatformName())
                    .tagline(request.getTagline())
                    .logoUrl(logoUrl)
                    .avatarUrl(request.getAvatarUrl())
                    .homepageType("fancy")
                    .createdAt(LocalDateTime.now())
                    .updatedAt(LocalDateTime.now())
                    .build();
            appConfigRepository.save(appConfig);
            log.info("App config saved in Auth: {}", appConfig.getAppName());
        } else {
            appConfig.setAppName(request.getPlatformName());
            appConfig.setTagline(request.getTagline());
            appConfig.setLogoUrl(logoUrl);
            appConfig.setAvatarUrl(request.getAvatarUrl());
            if (appConfig.getHomepageType() == null) {
                appConfig.setHomepageType("fancy");
            }
            appConfig.setUpdatedAt(LocalDateTime.now());
            appConfigRepository.save(appConfig);
            log.info("App config updated in Auth: {}", appConfig.getAppName());
        }

        // Check if user already exists in Auth
        User user = userRepository.findByUsername(request.getUsername())
                .or(() -> userRepository.findByEmail(request.getEmail()))
                .orElse(null);

        UserDTO userDTO;
        if (user == null) {
            // Create user as owner (no platformId at this level)
            userDTO = userService.registerUser(
                    request.getUsername(),
                    request.getEmail(),
                    request.getPassword(),
                    request.getName(),
                    null
            );

            // Update user role to owner and set avatar
            user = userRepository.findById(userDTO.getId())
                    .orElseThrow(() -> new RuntimeException("User not found after registration"));
            user.setRole("owner");
            if (request.getAvatarUrl() != null) {
                user.setAvatarUrl(request.getAvatarUrl());
                userDTO.setAvatarUrl(request.getAvatarUrl());
            }
            userRepository.save(user);
            log.info("First user created as owner in Auth: {}", user.getUsername());
        } else {
            log.info("Owner user already exists in Auth: {}", user.getUsername());
            
            // Set role as owner just in case it wasn't
            user.setRole("owner");
            if (request.getAvatarUrl() != null) {
                user.setAvatarUrl(request.getAvatarUrl());
            }
            userRepository.save(user);

            // Generate UserDTO for existing user
            userDTO = UserDTO.builder()
                    .id(user.getId())
                    .username(user.getUsername())
                    .email(user.getEmail())
                    .name(user.getName())
                    .role(user.getRole())
                    .avatarUrl(user.getAvatarUrl())
                    .build();
        }

        String token = jwtTokenProvider.generateToken(
                user.getId(),
                user.getUsername(),
                user.getRole(),
                user.getPlatformId()
        );

        userDTO.setRole("owner");

        log.info("Setup completed successfully in Auth");

        return AuthResponse.builder()
                .token(token)
                .user(userDTO)
                .expiresIn(86400)
                .build();
    }
}
