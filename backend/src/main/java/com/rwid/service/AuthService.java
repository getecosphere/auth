package com.rwid.service;

import com.rwid.dto.AuthRequest;
import com.rwid.dto.AuthResponse;
import com.rwid.dto.UserDTO;
import com.rwid.exception.ResourceNotFoundException;
import com.rwid.model.User;
import com.rwid.security.JwtTokenProvider;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

@Slf4j
@Service
public class AuthService {

    private final UserService userService;
    private final JwtTokenProvider jwtTokenProvider;

    public AuthService(UserService userService, JwtTokenProvider jwtTokenProvider) {
        this.userService = userService;
        this.jwtTokenProvider = jwtTokenProvider;
    }

    public AuthResponse login(AuthRequest request) {
        log.info("Login attempt for user: {}", request.getUsername());
        
        User user;
        try {
            user = userService.getUserEntityByUsername(request.getUsername());
        } catch (ResourceNotFoundException e) {
            log.warn("User not found: {}", request.getUsername());
            throw new IllegalArgumentException("Invalid credentials");
        }
        
        if (!userService.validatePassword(request.getPassword(), user.getPasswordHash())) {
            log.warn("Invalid password for user: {}", request.getUsername());
            throw new IllegalArgumentException("Invalid credentials");
        }

        String token = jwtTokenProvider.generateToken(
                user.getId(),
                user.getUsername(),
                user.getRole(),
                user.getPlatformId()
        );

        UserDTO userDTO = userService.getUserById(user.getId());

        log.info("User logged in successfully: {}", request.getUsername());
        
        return AuthResponse.builder()
                .token(token)
                .user(userDTO)
                .expiresIn(86400) // 24 hours in seconds
                .build();
    }

    public AuthResponse register(String username, String email, String password, String name, String platformId) {
        log.info("Registration attempt for user: {}", username);
        
        UserDTO userDTO = userService.registerUser(username, email, password, name, platformId);
        
        String token = jwtTokenProvider.generateToken(
                userDTO.getId(),
                userDTO.getUsername(),
                userDTO.getRole(),
                userDTO.getPlatformId()
        );

        log.info("User registered successfully: {}", username);
        
        return AuthResponse.builder()
                .token(token)
                .user(userDTO)
                .expiresIn(86400) // 24 hours in seconds
                .build();
    }

    public AuthResponse registerWithProfile(String email, String password, String name, 
                                           String whatsappNumber, String province, String platformId) {
        log.info("Registration attempt for user: {} with profile data", email);

        // Use default platform if not provided
        String finalPlatformId = (platformId == null || platformId.isEmpty()) ? "default" : platformId;

        UserDTO userDTO = userService.registerUserWithProfile(email, password, name, 
                                                             whatsappNumber, province, finalPlatformId);

        String token = jwtTokenProvider.generateToken(
                userDTO.getId(),
                userDTO.getUsername(),
                userDTO.getRole(),
                userDTO.getPlatformId()
        );

        log.info("User registered successfully with profile: {}", email);



        return AuthResponse.builder()
                .token(token)
                .user(userDTO)
                .expiresIn(86400)
                .build();
    }
}
