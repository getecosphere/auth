package com.rwid.controller;

import com.rwid.service.FileService;
import com.rwid.service.UserService;
import lombok.extern.slf4j.Slf4j;
import org.springframework.http.HttpHeaders;
import org.springframework.http.HttpStatus;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.security.access.prepost.PreAuthorize;
import org.springframework.web.bind.annotation.*;
import org.springframework.web.multipart.MultipartFile;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;

@Slf4j
@RestController
@RequestMapping("/files")
public class FileController {

    private final FileService fileService;
    private final UserService userService;

    public FileController(FileService fileService, UserService userService) {
        this.fileService = fileService;
        this.userService = userService;
    }

    @PostMapping("/upload")
    public ResponseEntity<Map<String, String>> uploadFile(
            @RequestParam MultipartFile file,
            @RequestParam String type,
            @RequestParam(required = false) String userId) {
        
        try {
            log.info("Uploading file: {} for type: {} with userId: {}", file.getOriginalFilename(), type, userId);
            log.info("File size: {} bytes", file.getSize());
            
            // For setup logo, userId is optional - use "system" as default
            String finalUserId = userId != null && !userId.isEmpty() ? userId : "system";
            
            // Verify user exists only if not system/setup
            if (!"system".equals(finalUserId) && !"setup".equals(finalUserId)) {
                userService.getUserById(finalUserId);
            }
            
            String fileUrl = fileService.uploadFile(file, type, finalUserId);
            log.info("File uploaded successfully: {}", fileUrl);
            
            Map<String, String> response = new HashMap<>();
            response.put("fileUrl", fileUrl);
            response.put("fileId", fileUrl.substring(fileUrl.lastIndexOf("/") + 1));
            
            return ResponseEntity.status(HttpStatus.CREATED).body(response);
        } catch (IOException e) {
            log.error("File upload failed", e);
            throw new RuntimeException("File upload failed: " + e.getMessage());
        } catch (Exception e) {
            log.error("Unexpected error during file upload", e);
            throw new RuntimeException("Unexpected error: " + e.getMessage());
        }
    }

    @GetMapping("/{fileId}")
    public ResponseEntity<byte[]> downloadFile(@PathVariable String fileId) {
        try {
            log.info("Downloading file: {}", fileId);
            byte[] fileContent = fileService.downloadFile(fileId);
            String contentType = fileService.getContentType(fileId);
            
            // If it's an image, serve inline; otherwise as attachment
            if (contentType.startsWith("image/")) {
                return ResponseEntity.ok()
                        .contentType(MediaType.parseMediaType(contentType))
                        .body(fileContent);
            } else {
                return ResponseEntity.ok()
                        .header(HttpHeaders.CONTENT_DISPOSITION, "attachment; filename=\"file\"")
                        .contentType(MediaType.APPLICATION_OCTET_STREAM)
                        .body(fileContent);
            }
        } catch (IOException e) {
            log.error("File download failed", e);
            throw new RuntimeException("File download failed: " + e.getMessage());
        }
    }

    @GetMapping("/view/{fileId}")
    public ResponseEntity<byte[]> viewFile(@PathVariable String fileId) {
        try {
            log.info("Viewing file: {}", fileId);
            byte[] fileContent = fileService.downloadFile(fileId);
            String contentType = fileService.getContentType(fileId);
            
            return ResponseEntity.ok()
                    .contentType(MediaType.parseMediaType(contentType))
                    .body(fileContent);
        } catch (IOException e) {
            log.error("File view failed", e);
            throw new RuntimeException("File view failed: " + e.getMessage());
        }
    }

    @DeleteMapping("/{fileId}")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<Void> deleteFile(@PathVariable String fileId) {
        try {
            log.info("Deleting file: {}", fileId);
            fileService.deleteFile(fileId);
            return ResponseEntity.noContent().build();
        } catch (IOException e) {
            log.error("File deletion failed", e);
            throw new RuntimeException("File deletion failed: " + e.getMessage());
        }
    }
}
