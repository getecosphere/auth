package com.rwid.dto;

import jakarta.validation.constraints.NotBlank;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

@Data
@NoArgsConstructor
@AllArgsConstructor
@Builder
public class AuthRequest {
    
    @NotBlank(message = "Username is required")
    private String username;
    
    @NotBlank(message = "Password is required")
    private String password;
}

@Data
@NoArgsConstructor
@AllArgsConstructor
@Builder
class RegisterRequest {
    @NotBlank(message = "Username is required")
    private String username;
    
    @NotBlank(message = "Email is required")
    private String email;
    
    @NotBlank(message = "Password is required")
    private String password;
    
    @NotBlank(message = "Name is required")
    private String name;
    
    @NotBlank(message = "WhatsApp number is required")
    private String whatsappNumber;
    
    private String province;
    
    private String platformId;
}
