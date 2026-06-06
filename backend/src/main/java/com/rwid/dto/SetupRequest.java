package com.rwid.dto;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

@Data
@NoArgsConstructor
@AllArgsConstructor
@Builder
public class SetupRequest {
    private String username;
    private String email;
    private String password;
    private String name;
    private String platformName;
    private String tagline;
    private String logoUrl;
    private String avatarUrl;
}
