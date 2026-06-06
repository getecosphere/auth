package com.rwid.controller;

import com.rwid.dto.AuthResponse;
import com.rwid.dto.SetupRequest;
import com.rwid.dto.SetupStatusResponse;
import com.rwid.service.SetupService;
import jakarta.validation.Valid;
import lombok.extern.slf4j.Slf4j;
import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

@Slf4j
@RestController
@RequestMapping("/setup")
public class SetupController {

    private final SetupService setupService;

    public SetupController(SetupService setupService) {
        this.setupService = setupService;
    }

    @GetMapping("/status")
    public ResponseEntity<SetupStatusResponse> getSetupStatus() {
        log.info("Checking setup status");
        SetupStatusResponse response = setupService.getSetupStatus();
        return ResponseEntity.ok(response);
    }

    @PostMapping("/complete")
    public ResponseEntity<AuthResponse> completeSetup(@Valid @RequestBody SetupRequest request) {
        log.info("Completing setup for platform: {}", request.getPlatformName());
        AuthResponse response = setupService.completeSetup(request);
        return ResponseEntity.status(HttpStatus.CREATED).body(response);
    }
}
