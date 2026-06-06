package com.rwid.dto;

import com.fasterxml.jackson.annotation.JsonInclude;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.time.LocalDateTime;
import java.util.List;

@Data
@NoArgsConstructor
@AllArgsConstructor
@Builder
@JsonInclude(JsonInclude.Include.NON_NULL)
public class UserDTO {
    
    private String id;
    private String name;
    private String username;
    private String email;
    private String headline;
    private String avatarUrl;
    private String coverPhotoUrl;
    private String bio;
    private String location;
    private String website;
    private String school;
    private String whatsappNumber;
    private String province;
    private String role;
    private String platformId;
    private List<String> interests;
    private List<ExperienceDTO> experiences;
    private List<EducationDTO> education;
    private List<String> skills;
    private List<CertificationDTO> certifications;
    private SocialLinksDTO socialLinks;
    private LocalDateTime createdAt;
    private LocalDateTime updatedAt;
    
    @Data
    @NoArgsConstructor
    @AllArgsConstructor
    @Builder
    @JsonInclude(JsonInclude.Include.NON_NULL)
    public static class ExperienceDTO {
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
    @JsonInclude(JsonInclude.Include.NON_NULL)
    public static class EducationDTO {
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
    @JsonInclude(JsonInclude.Include.NON_NULL)
    public static class CertificationDTO {
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
    @JsonInclude(JsonInclude.Include.NON_NULL)
    public static class SocialLinksDTO {
        private String linkedin;
        private String twitter;
        private String github;
        private String portfolio;
    }
}
