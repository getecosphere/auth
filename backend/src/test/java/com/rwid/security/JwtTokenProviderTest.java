package com.rwid.security;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.context.SpringBootTest;
import org.springframework.test.context.ActiveProfiles;

import static org.junit.jupiter.api.Assertions.*;

@SpringBootTest
@ActiveProfiles("test")
class JwtTokenProviderTest {

    @Autowired
    private JwtTokenProvider jwtTokenProvider;

    private String testToken;

    @BeforeEach
    void setUp() {
        testToken = jwtTokenProvider.generateToken("user-123", "testuser", "member", "platform-123");
    }

    @Test
    void testGenerateToken() {
        assertNotNull(testToken);
        assertFalse(testToken.isEmpty());
    }

    @Test
    void testValidateToken() {
        boolean isValid = jwtTokenProvider.validateToken(testToken);
        assertTrue(isValid);
    }

    @Test
    void testValidateInvalidToken() {
        boolean isValid = jwtTokenProvider.validateToken("invalid-token");
        assertFalse(isValid);
    }

    @Test
    void testGetUserIdFromToken() {
        String userId = jwtTokenProvider.getUserIdFromToken(testToken);
        assertEquals("user-123", userId);
    }

    @Test
    void testGetUsernameFromToken() {
        String username = jwtTokenProvider.getUsernameFromToken(testToken);
        assertEquals("testuser", username);
    }

    @Test
    void testGetRoleFromToken() {
        String role = jwtTokenProvider.getRoleFromToken(testToken);
        assertEquals("member", role);
    }

    @Test
    void testGetPlatformIdFromToken() {
        String platformId = jwtTokenProvider.getPlatformIdFromToken(testToken);
        assertEquals("platform-123", platformId);
    }

    @Test
    void testIsTokenExpired() {
        boolean isExpired = jwtTokenProvider.isTokenExpired(testToken);
        assertFalse(isExpired);
    }
}
