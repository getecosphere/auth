package com.rwid.controller;

import com.rwid.service.FileService;
import lombok.extern.slf4j.Slf4j;
import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;
import org.springframework.web.multipart.MultipartFile;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;

@Slf4j
@RestController
@RequestMapping("/setup")
public class SetupFileController {

    private final FileService fileService;

    public SetupFileController(FileService fileService) {
        this.fileService = fileService;
    }

    @PostMapping("/upload-logo")
    public ResponseEntity<Map<String, String>> uploadLogo(
            @RequestParam MultipartFile file) {
        
        try {
            log.info("Uploading setup logo: {} ({} bytes)", file.getOriginalFilename(), file.getSize());
            
            String fileUrl = fileService.uploadFile(file, "logo", "setup");
            log.info("Logo uploaded successfully: {}", fileUrl);
            
            Map<String, String> response = new HashMap<>();
            response.put("fileUrl", fileUrl);
            response.put("fileId", fileUrl.substring(fileUrl.lastIndexOf("/") + 1));
            
            return ResponseEntity.status(HttpStatus.CREATED).body(response);
        } catch (IOException e) {
            log.error("Logo upload failed", e);
            throw new RuntimeException("Logo upload failed: " + e.getMessage());
        } catch (Exception e) {
            log.error("Unexpected error during logo upload", e);
            throw new RuntimeException("Unexpected error: " + e.getMessage());
        }
    }

    @PostMapping("/upload-avatar")
    public ResponseEntity<Map<String, String>> uploadAvatar(
            @RequestParam MultipartFile file) {
        
        try {
            log.info("Uploading setup avatar: {} ({} bytes)", file.getOriginalFilename(), file.getSize());
            
            String fileUrl = fileService.uploadFile(file, "avatars", "setup");
            log.info("Avatar uploaded successfully: {}", fileUrl);
            
            Map<String, String> response = new HashMap<>();
            response.put("fileUrl", fileUrl);
            response.put("fileId", fileUrl.substring(fileUrl.lastIndexOf("/") + 1));
            
            return ResponseEntity.status(HttpStatus.CREATED).body(response);
        } catch (IOException e) {
            log.error("Avatar upload failed", e);
            throw new RuntimeException("Avatar upload failed: " + e.getMessage());
        } catch (Exception e) {
            log.error("Unexpected error during avatar upload", e);
            throw new RuntimeException("Unexpected error: " + e.getMessage());
        }
    }
}
