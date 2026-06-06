package com.rwid.service;

import com.rwid.dto.UserDTO;
import com.rwid.exception.ResourceNotFoundException;
import com.rwid.model.User;
import com.rwid.repository.UserRepository;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.InjectMocks;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;
import org.springframework.security.crypto.password.PasswordEncoder;

import java.time.LocalDateTime;
import java.util.Optional;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.*;

@ExtendWith(MockitoExtension.class)
class UserServiceTest {

    @Mock
    private UserRepository userRepository;

    @Mock
    private PasswordEncoder passwordEncoder;

    @InjectMocks
    private UserService userService;

    private User testUser;

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
    }

    @Test
    void testRegisterUserSuccess() {
        when(userRepository.findByUsername("newuser")).thenReturn(Optional.empty());
        when(userRepository.findByEmail("new@example.com")).thenReturn(Optional.empty());
        when(passwordEncoder.encode("password123")).thenReturn("hashed-password");
        when(userRepository.save(any(User.class))).thenReturn(testUser);

        UserDTO result = userService.registerUser("newuser", "new@example.com", "password123", "New User", "platform-123");

        assertNotNull(result);
        assertEquals("testuser", result.getUsername());
        assertEquals("test@example.com", result.getEmail());
        verify(userRepository, times(1)).save(any(User.class));
    }

    @Test
    void testRegisterUserWithDuplicateUsername() {
        when(userRepository.findByUsername("testuser")).thenReturn(Optional.of(testUser));

        UserDTO result = userService.registerUser("testuser", "new@example.com", "password123", "New User", "platform-123");
        assertNotNull(result);
        assertEquals("testuser", result.getUsername());
    }

    @Test
    void testGetUserByIdSuccess() {
        when(userRepository.findById("user-123")).thenReturn(Optional.of(testUser));

        UserDTO result = userService.getUserById("user-123");

        assertNotNull(result);
        assertEquals("testuser", result.getUsername());
        assertEquals("test@example.com", result.getEmail());
    }

    @Test
    void testGetUserByIdNotFound() {
        when(userRepository.findById("nonexistent")).thenReturn(Optional.empty());

        assertThrows(ResourceNotFoundException.class, () ->
                userService.getUserById("nonexistent")
        );
    }

    @Test
    void testGetUserByIdSoftDeleted() {
        testUser.setDeletedAt(LocalDateTime.now());
        when(userRepository.findById("user-123")).thenReturn(Optional.of(testUser));

        assertThrows(ResourceNotFoundException.class, () ->
                userService.getUserById("user-123")
        );
    }

    @Test
    void testUpdateUserSuccess() {
        when(userRepository.findById("user-123")).thenReturn(Optional.of(testUser));
        when(userRepository.save(any(User.class))).thenReturn(testUser);

        UserDTO result = userService.updateUser("user-123", "Updated Name", null, "New Bio", null, null, null, null, null);

        assertNotNull(result);
        verify(userRepository, times(1)).save(any(User.class));
    }

    @Test
    void testDeleteUserSuccess() {
        when(userRepository.findById("user-123")).thenReturn(Optional.of(testUser));
        when(userRepository.save(any(User.class))).thenReturn(testUser);

        userService.deleteUser("user-123");

        verify(userRepository, times(1)).save(any(User.class));
    }

    @Test
    void testValidatePasswordSuccess() {
        when(passwordEncoder.matches("password123", "hashed-password")).thenReturn(true);

        boolean result = userService.validatePassword("password123", "hashed-password");

        assertTrue(result);
    }

    @Test
    void testValidatePasswordFailure() {
        when(passwordEncoder.matches("wrongpassword", "hashed-password")).thenReturn(false);

        boolean result = userService.validatePassword("wrongpassword", "hashed-password");

        assertFalse(result);
    }
}
