package com.rwid.security;

import lombok.AllArgsConstructor;
import lombok.Getter;

@Getter
@AllArgsConstructor
public class JwtAuthenticationDetails {
    private String userId;
    private String username;
    private String role;
}
