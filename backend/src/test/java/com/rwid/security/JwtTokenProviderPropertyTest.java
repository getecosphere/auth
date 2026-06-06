package com.rwid.security;

import net.jqwik.api.*;
import net.jqwik.api.constraints.StringLength;
import org.junit.jupiter.api.BeforeEach;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.context.SpringBootTest;
import org.springframework.test.context.ActiveProfiles;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Property-based tests for JWT token validity
 * **Validates: Requirements 1.1, 1.3, 1.4**
 */
@SpringBootTest
@ActiveProfiles("test")
class JwtTokenProviderPropertyTest {

    @Autowired
    private JwtTokenProvider jwtTokenProvider;

    @BeforeEach
    void setUp() {
        assertNotNull(jwtTokenProvider);
    }

    /**
     * Property 1: Valid credentials grant access
     * For any valid userId, username, role, and platformId, a generated token should be valid
     * **Validates: Requirements 1.1**
     */
    @Property
    @Label("Valid credentials grant access")
    void validCredentialsGrantAccess(
            @ForAll @StringLength(min = 1, max = 50) String userId,
            @ForAll @StringLength(min = 1, max = 50) String username,
            @ForAll @StringLength(min = 1, max = 50) String role,
            @ForAll @StringLength(min = 1, max = 50) String platformId) {
        
        // Generate token with valid credentials
        String token = jwtTokenProvider.generateToken(userId, username, role, platformId);
        
        // Token should not be null or empty
        assertNotNull(token);
        assertFalse(token.isEmpty());
        
        // Token should be valid
        assertTrue(jwtTokenProvider.validateToken(token));
        
        // Token should not be expired
        assertFalse(jwtTokenProvider.isTokenExpired(token));
        
        // Token should contain the correct claims
        assertEquals(userId, jwtTokenProvider.getUserIdFromToken(token));
        assertEquals(username, jwtTokenProvider.getUsernameFromToken(token));
        assertEquals(role, jwtTokenProvider.getRoleFromToken(token));
        assertEquals(platformId, jwtTokenProvider.getPlatformIdFromToken(token));
    }

    /**
     * Property 3: Valid JWT tokens grant access
     * For any valid JWT token, the system should validate the token and grant access
     * **Validates: Requirements 1.3**
     */
    @Property
    @Label("Valid JWT tokens grant access")
    void validJwtTokensGrantAccess(
            @ForAll @StringLength(min = 1, max = 50) String userId,
            @ForAll @StringLength(min = 1, max = 50) String username,
            @ForAll @StringLength(min = 1, max = 50) String role,
            @ForAll @StringLength(min = 1, max = 50) String platformId) {
        
        // Generate a valid token
        String token = jwtTokenProvider.generateToken(userId, username, role, platformId);
        
        // Validate the token
        boolean isValid = jwtTokenProvider.validateToken(token);
        
        // Token should be valid
        assertTrue(isValid);
        
        // Should be able to extract all claims from valid token
        assertDoesNotThrow(() -> {
            jwtTokenProvider.getUserIdFromToken(token);
            jwtTokenProvider.getUsernameFromToken(token);
            jwtTokenProvider.getRoleFromToken(token);
            jwtTokenProvider.getPlatformIdFromToken(token);
        });
    }

    /**
     * Property 4: Expired tokens are rejected
     * For any JWT token that has expired, the system should reject the request
     * **Validates: Requirements 1.4**
     */
    @Property
    @Label("Expired tokens are rejected")
    void expiredTokensAreRejected(
            @ForAll @StringLength(min = 1, max = 50) String userId,
            @ForAll @StringLength(min = 1, max = 50) String username,
            @ForAll @StringLength(min = 1, max = 50) String role,
            @ForAll @StringLength(min = 1, max = 50) String platformId) {
        
        // Generate a valid token
        String token = jwtTokenProvider.generateToken(userId, username, role, platformId);
        
        // Token should not be expired immediately after generation
        assertFalse(jwtTokenProvider.isTokenExpired(token));
        
        // Invalid tokens should be rejected
        String invalidToken = "invalid.token.here";
        assertFalse(jwtTokenProvider.validateToken(invalidToken));
    }

    /**
     * Property: Invalid tokens are rejected
     * For any invalid token format, the system should reject validation
     * **Validates: Requirements 1.1**
     */
    @Property
    @Label("Invalid tokens are rejected")
    void invalidTokensAreRejected(
            @ForAll @StringLength(min = 1, max = 100) String invalidToken) {
        
        // Invalid tokens should fail validation
        boolean isValid = jwtTokenProvider.validateToken(invalidToken);
        
        // Should be false for any invalid token
        assertFalse(isValid);
    }

    /**
     * Property: Token claims are immutable
     * For any generated token, the claims should remain consistent across multiple reads
     * **Validates: Requirements 1.3**
     */
    @Property
    @Label("Token claims are immutable")
    void tokenClaimsAreImmutable(
            @ForAll @StringLength(min = 1, max = 50) String userId,
            @ForAll @StringLength(min = 1, max = 50) String username,
            @ForAll @StringLength(min = 1, max = 50) String role,
            @ForAll @StringLength(min = 1, max = 50) String platformId) {
        
        // Generate a token
        String token = jwtTokenProvider.generateToken(userId, username, role, platformId);
        
        // Read claims multiple times
        String userId1 = jwtTokenProvider.getUserIdFromToken(token);
        String userId2 = jwtTokenProvider.getUserIdFromToken(token);
        
        String username1 = jwtTokenProvider.getUsernameFromToken(token);
        String username2 = jwtTokenProvider.getUsernameFromToken(token);
        
        String role1 = jwtTokenProvider.getRoleFromToken(token);
        String role2 = jwtTokenProvider.getRoleFromToken(token);
        
        String platformId1 = jwtTokenProvider.getPlatformIdFromToken(token);
        String platformId2 = jwtTokenProvider.getPlatformIdFromToken(token);
        
        // Claims should be consistent
        assertEquals(userId1, userId2);
        assertEquals(username1, username2);
        assertEquals(role1, role2);
        assertEquals(platformId1, platformId2);
    }
}
