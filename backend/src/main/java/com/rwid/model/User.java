package com.rwid.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;
import org.springframework.data.annotation.Id;
import org.springframework.data.mongodb.core.index.Indexed;
import org.springframework.data.mongodb.core.mapping.Document;

import java.time.LocalDateTime;
import java.util.List;

@Data
@NoArgsConstructor
@AllArgsConstructor
@Builder
@Document(collection = "users")
public class User {
    
    @Id
    private String id;
    
    @Indexed(unique = true)
    private String username;
    
    @Indexed(unique = true)
    private String email;
    
    private String passwordHash;
    
    private String name;
    
    private String headline; // e.g., "Software Engineer at Company"
    
    private String avatarUrl;
    
    private String coverPhotoUrl;
    
    private String bio;
    
    private String location;
    
    private String website;
    
    private String school;
    
    private String whatsappNumber;
    
    private String province;
    
    private String role; // owner, mentor, member, visitor
    
    private String platformId;
    
    private List<String> interests;
    
    private List<Experience> experiences;
    
    private List<Education> education;
    
    private List<String> skills;
    
    private List<Certification> certifications;
    
    private SocialLinks socialLinks;
    
    private LocalDateTime createdAt;
    
    private LocalDateTime updatedAt;
    
    private LocalDateTime deletedAt; // soft delete
    
    @Data
    @NoArgsConstructor
    @AllArgsConstructor
    @Builder
    public static class Experience {
        private String id;
        private String title;
        private String company;
        private String location;
        private String description;
        private LocalDateTime startDate;
        private LocalDateTime endDate;
        private Boolean currentlyWorking;
    }
    
    @Data
    @NoArgsConstructor
    @AllArgsConstructor
    @Builder
    public static class Education {
        private String id;
        private String school;
        private String degree;
        private String fieldOfStudy;
        private LocalDateTime startDate;
        private LocalDateTime endDate;
        private String description;
    }
    
    @Data
    @NoArgsConstructor
    @AllArgsConstructor
    @Builder
    public static class Certification {
        private String id;
        private String name;
        private String issuer;
        private LocalDateTime issueDate;
        private LocalDateTime expirationDate;
        private String credentialUrl;
    }
    
    @Data
    @NoArgsConstructor
    @AllArgsConstructor
    @Builder
    public static class SocialLinks {
        private String linkedin;
        private String twitter;
        private String github;
        private String portfolio;
    }
}
