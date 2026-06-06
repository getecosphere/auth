package com.rwid.controller;

import com.rwid.dto.UserDTO;
import com.rwid.service.FileService;
import com.rwid.service.UserService;
import lombok.extern.slf4j.Slf4j;
import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.security.access.prepost.PreAuthorize;
import org.springframework.web.bind.annotation.*;
import org.springframework.web.multipart.MultipartFile;

import java.io.IOException;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

@Slf4j
@RestController
@RequestMapping("/users")
public class UserController {

    private final UserService userService;
    private final FileService fileService;

    public UserController(UserService userService, FileService fileService) {
        this.userService = userService;
        this.fileService = fileService;
    }

    @GetMapping
    public ResponseEntity<List<UserDTO>> getAllUsers() {
        log.info("Fetching all users");
        List<UserDTO> users = userService.getAllUsers();
        return ResponseEntity.ok(users);
    }

    @GetMapping("/username/{username}")
    public ResponseEntity<UserDTO> getUserByUsername(@PathVariable String username) {
        log.info("Fetching user by username: {}", username);
        UserDTO user = userService.getUserByUsername(username);
        return ResponseEntity.ok(user);
    }

    @GetMapping("/{userId}")
    public ResponseEntity<UserDTO> getUserById(@PathVariable String userId) {
        log.info("Fetching user: {}", userId);
        UserDTO user = userService.getUserById(userId);
        return ResponseEntity.ok(user);
    }

    @PutMapping("/{userId}")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<UserDTO> updateUser(
            @PathVariable String userId,
            @RequestParam(required = false) String name,
            @RequestParam(required = false) String headline,
            @RequestParam(required = false) String bio,
            @RequestParam(required = false) String location,
            @RequestParam(required = false) String website,
            @RequestParam(required = false) List<String> interests,
            @RequestParam(required = false) String avatarUrl,
            @RequestParam(required = false) String coverPhotoUrl) {
        
        log.info("Updating user: {}", userId);
        UserDTO updatedUser = userService.updateUser(userId, name, headline, bio, location, website, interests, avatarUrl, coverPhotoUrl);
        return ResponseEntity.ok(updatedUser);
    }

    @DeleteMapping("/{userId}")
    @PreAuthorize("hasRole('OWNER')")
    public ResponseEntity<Void> deleteUser(@PathVariable String userId) {
        log.info("Deleting user: {}", userId);
        userService.deleteUser(userId);
        return ResponseEntity.noContent().build();
    }

    @GetMapping("/search")
    public ResponseEntity<List<UserDTO>> searchUsers(
            @RequestParam String platformId,
            @RequestParam String query) {
        
        log.info("Searching users in platform: {} with query: {}", platformId, query);
        List<UserDTO> users = userService.searchUsers(platformId, query);
        return ResponseEntity.ok(users);
    }

    @GetMapping("/platform/{platformId}")
    public ResponseEntity<List<UserDTO>> getUsersByPlatform(@PathVariable String platformId) {
        log.info("Fetching users for platform: {}", platformId);
        List<UserDTO> users = userService.getUsersByPlatform(platformId);
        return ResponseEntity.ok(users);
    }

    @PostMapping("/{userId}/avatar")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<Map<String, String>> uploadAvatar(
            @PathVariable String userId,
            @RequestParam("file") MultipartFile file) {
        
        try {
            log.info("Uploading avatar for user: {}", userId);
            
            // Verify user exists
            userService.getUserById(userId);
            
            String fileUrl = fileService.uploadFile(file, "avatars", userId);
            
            // Update user with avatar URL
            userService.updateAvatar(userId, fileUrl);
            
            Map<String, String> response = new HashMap<>();
            response.put("avatarUrl", fileUrl);
            
            return ResponseEntity.status(HttpStatus.CREATED).body(response);
        } catch (IOException e) {
            log.error("Avatar upload failed", e);
            throw new RuntimeException("Avatar upload failed: " + e.getMessage());
        }
    }

    @PostMapping("/{userId}/upload-cover-photo")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<Map<String, String>> uploadCoverPhoto(
            @PathVariable String userId,
            @RequestParam("file") MultipartFile file) {
        
        try {
            log.info("Uploading cover photo for user: {}", userId);
            
            // Verify user exists
            userService.getUserById(userId);
            
            String fileUrl = fileService.uploadFile(file, "cover-photos", userId);
            
            // Update user with cover photo URL
            userService.updateCoverPhoto(userId, fileUrl);
            
            Map<String, String> response = new HashMap<>();
            response.put("coverPhotoUrl", fileUrl);
            
            return ResponseEntity.status(HttpStatus.CREATED).body(response);
        } catch (IOException e) {
            log.error("Cover photo upload failed", e);
            throw new RuntimeException("Cover photo upload failed: " + e.getMessage());
        }
    }
    
    @PostMapping("/{userId}/experience")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<UserDTO> addExperience(
            @PathVariable String userId,
            @RequestBody Map<String, Object> experienceData) {
        
        log.info("Adding experience for user: {}", userId);
        UserDTO updatedUser = userService.addExperience(userId, experienceData);
        return ResponseEntity.status(HttpStatus.CREATED).body(updatedUser);
    }
    
    @PutMapping("/{userId}/experience/{experienceId}")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<UserDTO> updateExperience(
            @PathVariable String userId,
            @PathVariable String experienceId,
            @RequestBody Map<String, Object> experienceData) {
        
        log.info("Updating experience {} for user: {}", experienceId, userId);
        UserDTO updatedUser = userService.updateExperience(userId, experienceId, experienceData);
        return ResponseEntity.ok(updatedUser);
    }
    
    @DeleteMapping("/{userId}/experience/{experienceId}")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<UserDTO> deleteExperience(
            @PathVariable String userId,
            @PathVariable String experienceId) {
        
        log.info("Deleting experience {} for user: {}", experienceId, userId);
        UserDTO updatedUser = userService.deleteExperience(userId, experienceId);
        return ResponseEntity.ok(updatedUser);
    }
    
    @PostMapping("/{userId}/education")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<UserDTO> addEducation(
            @PathVariable String userId,
            @RequestBody Map<String, Object> educationData) {
        
        log.info("Adding education for user: {}", userId);
        UserDTO updatedUser = userService.addEducation(userId, educationData);
        return ResponseEntity.status(HttpStatus.CREATED).body(updatedUser);
    }
    
    @PutMapping("/{userId}/education/{educationId}")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<UserDTO> updateEducation(
            @PathVariable String userId,
            @PathVariable String educationId,
            @RequestBody Map<String, Object> educationData) {
        
        log.info("Updating education {} for user: {}", educationId, userId);
        UserDTO updatedUser = userService.updateEducation(userId, educationId, educationData);
        return ResponseEntity.ok(updatedUser);
    }
    
    @DeleteMapping("/{userId}/education/{educationId}")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<UserDTO> deleteEducation(
            @PathVariable String userId,
            @PathVariable String educationId) {
        
        log.info("Deleting education {} for user: {}", educationId, userId);
        UserDTO updatedUser = userService.deleteEducation(userId, educationId);
        return ResponseEntity.ok(updatedUser);
    }
    
    @PostMapping("/{userId}/skills")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<UserDTO> addSkill(
            @PathVariable String userId,
            @RequestParam String skill) {
        
        log.info("Adding skill for user: {}", userId);
        UserDTO updatedUser = userService.addSkill(userId, skill);
        return ResponseEntity.status(HttpStatus.CREATED).body(updatedUser);
    }
    
    @DeleteMapping("/{userId}/skills/{skill}")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<UserDTO> removeSkill(
            @PathVariable String userId,
            @PathVariable String skill) {
        
        log.info("Removing skill for user: {}", userId);
        UserDTO updatedUser = userService.removeSkill(userId, skill);
        return ResponseEntity.ok(updatedUser);
    }
    
    @PostMapping("/{userId}/certifications")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<UserDTO> addCertification(
            @PathVariable String userId,
            @RequestBody Map<String, Object> certificationData) {
        
        log.info("Adding certification for user: {}", userId);
        UserDTO updatedUser = userService.addCertification(userId, certificationData);
        return ResponseEntity.status(HttpStatus.CREATED).body(updatedUser);
    }
    
    @PutMapping("/{userId}/certifications/{certificationId}")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<UserDTO> updateCertification(
            @PathVariable String userId,
            @PathVariable String certificationId,
            @RequestBody Map<String, Object> certificationData) {
        
        log.info("Updating certification {} for user: {}", certificationId, userId);
        UserDTO updatedUser = userService.updateCertification(userId, certificationId, certificationData);
        return ResponseEntity.ok(updatedUser);
    }
    
    @DeleteMapping("/{userId}/certifications/{certificationId}")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<UserDTO> deleteCertification(
            @PathVariable String userId,
            @PathVariable String certificationId) {
        
        log.info("Deleting certification {} for user: {}", certificationId, userId);
        UserDTO updatedUser = userService.deleteCertification(userId, certificationId);
        return ResponseEntity.ok(updatedUser);
    }
    
    @PutMapping("/{userId}/social-links")
    @PreAuthorize("hasAnyRole('OWNER', 'MENTOR', 'MEMBER')")
    public ResponseEntity<UserDTO> updateSocialLinks(
            @PathVariable String userId,
            @RequestBody Map<String, String> socialLinks) {
        
        log.info("Updating social links for user: {}", userId);
        UserDTO updatedUser = userService.updateSocialLinks(userId, socialLinks);
        return ResponseEntity.ok(updatedUser);
    }
}
