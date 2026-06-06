package com.rwid.service;

import com.rwid.dto.AuthRequest;
import com.rwid.dto.AuthResponse;
import com.rwid.dto.UserDTO;
import com.rwid.model.User;
import com.rwid.security.JwtTokenProvider;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;

import java.time.LocalDateTime;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.*;

@ExtendWith(MockitoExtension.class)
class AuthServiceTest {

    @Mock
    private UserService userService;

    @Mock
    private JwtTokenProvider jwtTokenProvider;

    @InjectMocks
    private AuthService authService;

    private User testUser;
    private UserDTO testUserDTO;

    @BeforeEach
    void setUp() {
        testUser = User.builder()
                .id("user-123")
                .username("testuser")
                .email("test@example.com")
                .passwordHash("hashed-password")
                .name("Test User")
                .role("member")
                .platformId("platform-123")
                .createdAt(LocalDateTime.now())
                .updatedAt(LocalDateTime.now())
                .build();

        testUserDTO = UserDTO.builder()
                .id("user-123")
                .username("testuser")
                .email("test@example.com")
                .name("Test User")
                .role("member")
                .platformId("platform-123")
                .build();
    }

    @Test
    void testLoginSuccess() {
        AuthRequest request = new AuthRequest("testuser", "password123");

        when(userService.getUserEntityByUsername("testuser")).thenReturn(testUser);
        when(userService.validatePassword("password123", "hashed-password")).thenReturn(true);
        when(jwtTokenProvider.generateToken("user-123", "testuser", "member", "platform-123"))
                .thenReturn("jwt-token");
        when(userService.getUserById("user-123")).thenReturn(testUserDTO);

        AuthResponse response = authService.login(request);

        assertNotNull(response);
        assertEquals("jwt-token", response.getToken());
        assertEquals("testuser", response.getUser().getUsername());
        assertEquals(86400, response.getExpiresIn());
    }

    @Test
    void testLoginWithInvalidPassword() {
        AuthRequest request = new AuthRequest("testuser", "wrongpassword");

        when(userService.getUserEntityByUsername("testuser")).thenReturn(testUser);
        when(userService.validatePassword("wrongpassword", "hashed-password")).thenReturn(false);

        assertThrows(IllegalArgumentException.class, () -> authService.login(request));
    }

    @Test
    void testRegisterSuccess() {
        when(userService.registerUser("newuser", "new@example.com", "password123", "New User", "platform-123"))
                .thenReturn(testUserDTO);
        when(jwtTokenProvider.generateToken("user-123", "testuser", "member", "platform-123"))
                .thenReturn("jwt-token");

        AuthResponse response = authService.register("newuser", "new@example.com", "password123", "New User", "platform-123");

        assertNotNull(response);
        assertEquals("jwt-token", response.getToken());
        assertEquals("testuser", response.getUser().getUsername());
        assertEquals(86400, response.getExpiresIn());
    }
}
