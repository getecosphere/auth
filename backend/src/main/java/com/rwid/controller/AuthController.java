package com.rwid.controller;

import com.rwid.dto.AuthRequest;
import com.rwid.dto.AuthResponse;
import com.rwid.service.AuthService;
import jakarta.validation.Valid;
import lombok.extern.slf4j.Slf4j;
import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

@Slf4j
@RestController
@RequestMapping("/auth")
public class AuthController {

    private final AuthService authService;

    public AuthController(AuthService authService) {
        this.authService = authService;
    }

    @PostMapping("/login")
    public ResponseEntity<AuthResponse> login(@Valid @RequestBody AuthRequest request) {
        log.info("Login request for user: {}", request.getUsername());
        AuthResponse response = authService.login(request);
        return ResponseEntity.ok(response);
    }

    @PostMapping("/register")
    public ResponseEntity<AuthResponse> register(
            @RequestParam String username,
            @RequestParam String email,
            @RequestParam String password,
            @RequestParam String name,
            @RequestParam String platformId,
            @RequestParam(required = false, defaultValue = "member") String role) {

        log.info("Registration request for user: {}", username);
        AuthResponse response = authService.register(username, email, password, name, platformId, role);
        return ResponseEntity.status(HttpStatus.CREATED).body(response);
    }

    @PostMapping("/register-with-profile")
    public ResponseEntity<AuthResponse> registerWithProfile(
            @RequestParam String email,
            @RequestParam String password,
            @RequestParam String name,
            @RequestParam String whatsappNumber,
            @RequestParam String province) {

        log.info("Registration with profile request for user: {}", email);
        AuthResponse response = authService.registerWithProfile(email, password, name, 
                                                               whatsappNumber, province, "");
        return ResponseEntity.status(HttpStatus.CREATED).body(response);
    }
}
