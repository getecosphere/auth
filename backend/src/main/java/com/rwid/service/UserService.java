package com.rwid.service;

import com.rwid.dto.UserDTO;
import com.rwid.exception.ResourceNotFoundException;
import com.rwid.model.User;
import com.rwid.repository.UserRepository;
import lombok.extern.slf4j.Slf4j;
import com.rwid.dto.UserUpdatedEvent;
import org.springframework.kafka.core.KafkaTemplate;
import org.springframework.security.crypto.password.PasswordEncoder;
import org.springframework.stereotype.Service;

import java.time.LocalDateTime;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

@Slf4j
@Service
public class UserService {

    private final UserRepository userRepository;
    private final PasswordEncoder passwordEncoder;
    private final KafkaTemplate<String, UserUpdatedEvent> kafkaTemplate;

    @org.springframework.beans.factory.annotation.Value("${app.kafka.enabled:true}")
    private boolean kafkaEnabled;

    public UserService(UserRepository userRepository, PasswordEncoder passwordEncoder,
                       KafkaTemplate<String, UserUpdatedEvent> kafkaTemplate) {
        this.userRepository = userRepository;
        this.passwordEncoder = passwordEncoder;
        this.kafkaTemplate = kafkaTemplate;
    }

    private void publishUserEvent(String eventType, UserDTO userDTO) {
        if (!kafkaEnabled) {
            log.info("Kafka publishing is disabled. Skipping event for user: {}", userDTO.getId());
            return;
        }
        try {
            UserUpdatedEvent event = UserUpdatedEvent.builder()
                    .eventId(java.util.UUID.randomUUID().toString())
                    .eventType(eventType)
                    .timestamp(LocalDateTime.now())
                    .user(userDTO)
                    .build();
            kafkaTemplate.send("user-profile-events", userDTO.getId(), event);
            log.info("Successfully published {} event to Kafka for user: {}", eventType, userDTO.getId());
        } catch (Exception e) {
            log.error("Failed to publish {} event to Kafka for user: {}", eventType, userDTO.getId(), e);
        }
    }

    public UserDTO registerUser(String username, String email, String password, String name, String platformId) {
        return registerUser(username, email, password, name, platformId, "member");
    }

    public UserDTO registerUser(String username, String email, String password, String name, String platformId, String role) {
        // Check if user already exists
        java.util.Optional<User> existingUsername = userRepository.findByUsername(username);
        if (existingUsername.isPresent()) {
            log.info("User with username {} already exists, returning existing user", username);
            return mapToDTO(existingUsername.get());
        }
        java.util.Optional<User> existingEmail = userRepository.findByEmail(email);
        if (existingEmail.isPresent()) {
            log.info("User with email {} already exists, returning existing user", email);
            return mapToDTO(existingEmail.get());
        }

        User user = User.builder()
                .username(username)
                .email(email)
                .passwordHash(passwordEncoder.encode(password))
                .name(name)
                .role(role != null && !role.isBlank() ? role : "member")
                .platformId(platformId)
                .createdAt(LocalDateTime.now())
                .updatedAt(LocalDateTime.now())
                .build();

        User savedUser = userRepository.save(user);
        log.info("User registered: {}", username);
        UserDTO dto = mapToDTO(savedUser);
        publishUserEvent("USER_CREATED", dto);
        return dto;
    }

    public UserDTO getUserById(String userId) {
        User user = userRepository.findById(userId)
                .orElseThrow(() -> new ResourceNotFoundException("User not found with id: " + userId));
        
        if (user.getDeletedAt() != null) {
            throw new ResourceNotFoundException("User not found with id: " + userId);
        }
        
        return mapToDTO(user);
    }

    public UserDTO getUserByUsername(String username) {
        User user = userRepository.findByUsername(username)
                .orElseThrow(() -> new ResourceNotFoundException("User not found with username: " + username));
        
        if (user.getDeletedAt() != null) {
            throw new ResourceNotFoundException("User not found with username: " + username);
        }
        
        return mapToDTO(user);
    }

    public User getUserEntityById(String userId) {
        User user = userRepository.findById(userId)
                .orElseThrow(() -> new ResourceNotFoundException("User not found with id: " + userId));
        
        if (user.getDeletedAt() != null) {
            throw new ResourceNotFoundException("User not found with id: " + userId);
        }
        
        return user;
    }

    public User getUserEntityByUsername(String username) {
        User user = userRepository.findByUsername(username)
                .orElseThrow(() -> new ResourceNotFoundException("User not found with username: " + username));
        
        if (user.getDeletedAt() != null) {
            throw new ResourceNotFoundException("User not found with username: " + username);
        }
        
        return user;
    }

    public UserDTO updateUser(String userId, String name, String headline, String bio, String location, String website, List<String> interests, String avatarUrl, String coverPhotoUrl) {
        User user = getUserEntityById(userId);
        
        if (name != null) user.setName(name);
        if (headline != null) user.setHeadline(headline);
        if (bio != null) user.setBio(bio);
        if (location != null) user.setLocation(location);
        if (website != null) user.setWebsite(website);
        if (interests != null) {
            user.setInterests(interests);
        }
        if (avatarUrl != null) user.setAvatarUrl(avatarUrl);
        if (coverPhotoUrl != null) user.setCoverPhotoUrl(coverPhotoUrl);
        
        user.setUpdatedAt(LocalDateTime.now());
        User updatedUser = userRepository.save(user);
        log.info("User updated: {}", userId);
        UserDTO dto = mapToDTO(updatedUser);
        publishUserEvent("USER_UPDATED", dto);
        return dto;
    }
    
    public void updateCoverPhoto(String userId, String coverPhotoUrl) {
        User user = getUserEntityById(userId);
        user.setCoverPhotoUrl(coverPhotoUrl);
        user.setUpdatedAt(LocalDateTime.now());
        User savedUser = userRepository.save(user);
        publishUserEvent("USER_UPDATED", mapToDTO(savedUser));
    }

    public void updateAvatar(String userId, String avatarUrl) {
        User user = getUserEntityById(userId);
        user.setAvatarUrl(avatarUrl);
        user.setUpdatedAt(LocalDateTime.now());
        User savedUser = userRepository.save(user);
        publishUserEvent("USER_UPDATED", mapToDTO(savedUser));
    }
    
    public UserDTO addExperience(String userId, Map<String, Object> experienceData) {
        User user = getUserEntityById(userId);
        if (user.getExperiences() == null) {
            user.setExperiences(new java.util.ArrayList<>());
        }
        
        User.Experience experience = User.Experience.builder()
                .id(java.util.UUID.randomUUID().toString())
                .title((String) experienceData.get("title"))
                .company((String) experienceData.get("company"))
                .location((String) experienceData.get("location"))
                .description((String) experienceData.get("description"))
                .startDate(parseDate(experienceData.get("startDate")))
                .endDate(parseDate(experienceData.get("endDate")))
                .currentlyWorking((Boolean) experienceData.getOrDefault("currentlyWorking", false))
                .build();
        
        user.getExperiences().add(experience);
        user.setUpdatedAt(LocalDateTime.now());
        User updatedUser = userRepository.save(user);
        UserDTO dto = mapToDTO(updatedUser);
        publishUserEvent("USER_UPDATED", dto);
        return dto;
    }
    
    public UserDTO updateExperience(String userId, String experienceId, Map<String, Object> experienceData) {
        User user = getUserEntityById(userId);
        if (user.getExperiences() != null) {
            user.getExperiences().stream()
                    .filter(e -> e.getId().equals(experienceId))
                    .findFirst()
                    .ifPresent(e -> {
                        if (experienceData.containsKey("title")) e.setTitle((String) experienceData.get("title"));
                        if (experienceData.containsKey("company")) e.setCompany((String) experienceData.get("company"));
                        if (experienceData.containsKey("location")) e.setLocation((String) experienceData.get("location"));
                        if (experienceData.containsKey("description")) e.setDescription((String) experienceData.get("description"));
                        if (experienceData.containsKey("startDate")) e.setStartDate(parseDate(experienceData.get("startDate")));
                        if (experienceData.containsKey("endDate")) e.setEndDate(parseDate(experienceData.get("endDate")));
                        if (experienceData.containsKey("currentlyWorking")) e.setCurrentlyWorking((Boolean) experienceData.get("currentlyWorking"));
                    });
        }
        user.setUpdatedAt(LocalDateTime.now());
        User updatedUser = userRepository.save(user);
        UserDTO dto = mapToDTO(updatedUser);
        publishUserEvent("USER_UPDATED", dto);
        return dto;
    }
    
    public UserDTO deleteExperience(String userId, String experienceId) {
        User user = getUserEntityById(userId);
        if (user.getExperiences() != null) {
            user.setExperiences(user.getExperiences().stream()
                    .filter(e -> !e.getId().equals(experienceId))
                    .collect(Collectors.toList()));
        }
        user.setUpdatedAt(LocalDateTime.now());
        User updatedUser = userRepository.save(user);
        UserDTO dto = mapToDTO(updatedUser);
        publishUserEvent("USER_UPDATED", dto);
        return dto;
    }
    
    public UserDTO addEducation(String userId, Map<String, Object> educationData) {
        User user = getUserEntityById(userId);
        if (user.getEducation() == null) {
            user.setEducation(new java.util.ArrayList<>());
        }
        
        User.Education education = User.Education.builder()
                .id(java.util.UUID.randomUUID().toString())
                .school((String) educationData.get("school"))
                .degree((String) educationData.get("degree"))
                .fieldOfStudy((String) educationData.get("fieldOfStudy"))
                .startDate(parseDate(educationData.get("startDate")))
                .endDate(parseDate(educationData.get("endDate")))
                .description((String) educationData.get("description"))
                .build();
        
        user.getEducation().add(education);
        user.setUpdatedAt(LocalDateTime.now());
        User updatedUser = userRepository.save(user);
        UserDTO dto = mapToDTO(updatedUser);
        publishUserEvent("USER_UPDATED", dto);
        return dto;
    }
    
    public UserDTO updateEducation(String userId, String educationId, Map<String, Object> educationData) {
        User user = getUserEntityById(userId);
        if (user.getEducation() != null) {
            user.getEducation().stream()
                    .filter(e -> e.getId().equals(educationId))
                    .findFirst()
                    .ifPresent(e -> {
                        if (educationData.containsKey("school")) e.setSchool((String) educationData.get("school"));
                        if (educationData.containsKey("degree")) e.setDegree((String) educationData.get("degree"));
                        if (educationData.containsKey("fieldOfStudy")) e.setFieldOfStudy((String) educationData.get("fieldOfStudy"));
                        if (educationData.containsKey("startDate")) e.setStartDate(parseDate(educationData.get("startDate")));
                        if (educationData.containsKey("endDate")) e.setEndDate(parseDate(educationData.get("endDate")));
                        if (educationData.containsKey("description")) e.setDescription((String) educationData.get("description"));
                    });
        }
        user.setUpdatedAt(LocalDateTime.now());
        User updatedUser = userRepository.save(user);
        UserDTO dto = mapToDTO(updatedUser);
        publishUserEvent("USER_UPDATED", dto);
        return dto;
    }
    
    public UserDTO deleteEducation(String userId, String educationId) {
        User user = getUserEntityById(userId);
        if (user.getEducation() != null) {
            user.setEducation(user.getEducation().stream()
                    .filter(e -> !e.getId().equals(educationId))
                    .collect(Collectors.toList()));
        }
        user.setUpdatedAt(LocalDateTime.now());
        User updatedUser = userRepository.save(user);
        UserDTO dto = mapToDTO(updatedUser);
        publishUserEvent("USER_UPDATED", dto);
        return dto;
    }
    
    public UserDTO addSkill(String userId, String skill) {
        User user = getUserEntityById(userId);
        if (user.getSkills() == null) {
            user.setSkills(new java.util.ArrayList<>());
        }
        if (!user.getSkills().contains(skill)) {
            user.getSkills().add(skill);
        }
        user.setUpdatedAt(LocalDateTime.now());
        User updatedUser = userRepository.save(user);
        UserDTO dto = mapToDTO(updatedUser);
        publishUserEvent("USER_UPDATED", dto);
        return dto;
    }
    
    public UserDTO removeSkill(String userId, String skill) {
        User user = getUserEntityById(userId);
        if (user.getSkills() != null) {
            user.setSkills(user.getSkills().stream()
                    .filter(s -> !s.equals(skill))
                    .collect(Collectors.toList()));
        }
        user.setUpdatedAt(LocalDateTime.now());
        User updatedUser = userRepository.save(user);
        UserDTO dto = mapToDTO(updatedUser);
        publishUserEvent("USER_UPDATED", dto);
        return dto;
    }
    
    public UserDTO addCertification(String userId, Map<String, Object> certificationData) {
        User user = getUserEntityById(userId);
        if (user.getCertifications() == null) {
            user.setCertifications(new java.util.ArrayList<>());
        }
        
        User.Certification certification = User.Certification.builder()
                .id(java.util.UUID.randomUUID().toString())
                .name((String) certificationData.get("name"))
                .issuer((String) certificationData.get("issuer"))
                .issueDate(parseDate(certificationData.get("issueDate")))
                .expirationDate(parseDate(certificationData.get("expirationDate")))
                .credentialUrl((String) certificationData.get("credentialUrl"))
                .build();
        
        user.getCertifications().add(certification);
        user.setUpdatedAt(LocalDateTime.now());
        User updatedUser = userRepository.save(user);
        UserDTO dto = mapToDTO(updatedUser);
        publishUserEvent("USER_UPDATED", dto);
        return dto;
    }
    
    public UserDTO updateCertification(String userId, String certificationId, Map<String, Object> certificationData) {
        User user = getUserEntityById(userId);
        if (user.getCertifications() != null) {
            user.getCertifications().stream()
                    .filter(c -> c.getId().equals(certificationId))
                    .findFirst()
                    .ifPresent(c -> {
                        if (certificationData.containsKey("name")) c.setName((String) certificationData.get("name"));
                        if (certificationData.containsKey("issuer")) c.setIssuer((String) certificationData.get("issuer"));
                        if (certificationData.containsKey("issueDate")) c.setIssueDate(parseDate(certificationData.get("issueDate")));
                        if (certificationData.containsKey("expirationDate")) c.setExpirationDate(parseDate(certificationData.get("expirationDate")));
                        if (certificationData.containsKey("credentialUrl")) c.setCredentialUrl((String) certificationData.get("credentialUrl"));
                    });
        }
        user.setUpdatedAt(LocalDateTime.now());
        User updatedUser = userRepository.save(user);
        UserDTO dto = mapToDTO(updatedUser);
        publishUserEvent("USER_UPDATED", dto);
        return dto;
    }
    
    public UserDTO deleteCertification(String userId, String certificationId) {
        User user = getUserEntityById(userId);
        if (user.getCertifications() != null) {
            user.setCertifications(user.getCertifications().stream()
                    .filter(c -> !c.getId().equals(certificationId))
                    .collect(Collectors.toList()));
        }
        user.setUpdatedAt(LocalDateTime.now());
        User updatedUser = userRepository.save(user);
        UserDTO dto = mapToDTO(updatedUser);
        publishUserEvent("USER_UPDATED", dto);
        return dto;
    }
    
    public UserDTO updateSocialLinks(String userId, Map<String, String> socialLinksData) {
        User user = getUserEntityById(userId);
        
        User.SocialLinks socialLinks = user.getSocialLinks();
        if (socialLinks == null) {
            socialLinks = new User.SocialLinks();
        }
        
        if (socialLinksData.containsKey("linkedin")) socialLinks.setLinkedin(socialLinksData.get("linkedin"));
        if (socialLinksData.containsKey("twitter")) socialLinks.setTwitter(socialLinksData.get("twitter"));
        if (socialLinksData.containsKey("github")) socialLinks.setGithub(socialLinksData.get("github"));
        if (socialLinksData.containsKey("portfolio")) socialLinks.setPortfolio(socialLinksData.get("portfolio"));
        
        user.setSocialLinks(socialLinks);
        user.setUpdatedAt(LocalDateTime.now());
        User updatedUser = userRepository.save(user);
        UserDTO dto = mapToDTO(updatedUser);
        publishUserEvent("USER_UPDATED", dto);
        return dto;
    }
    
    private LocalDateTime parseDate(Object dateObj) {
        if (dateObj == null) return null;
        if (dateObj instanceof LocalDateTime) return (LocalDateTime) dateObj;
        if (dateObj instanceof String) {
            String dateStr = (String) dateObj;
            try {
                // Try ISO format first (YYYY-MM-DD)
                return LocalDateTime.parse(dateStr + "T00:00:00");
            } catch (Exception e1) {
                try {
                    // Try DD/MM/YYYY format
                    String[] parts = dateStr.split("/");
                    if (parts.length == 3) {
                        int day = Integer.parseInt(parts[0]);
                        int month = Integer.parseInt(parts[1]);
                        int year = Integer.parseInt(parts[2]);
                        return LocalDateTime.of(year, month, day, 0, 0, 0);
                    }
                } catch (Exception e2) {
                    log.warn("Failed to parse date: {}", dateStr);
                }
            }
        }
        return null;
    }

    public void deleteUser(String userId) {
        User user = getUserEntityById(userId);
        user.setDeletedAt(LocalDateTime.now());
        User savedUser = userRepository.save(user);
        log.info("User deleted (soft): {}", userId);
        publishUserEvent("USER_DELETED", mapToDTO(savedUser));
    }

    public List<UserDTO> searchUsers(String platformId, String query) {
        List<User> users = userRepository.searchByNameInPlatform(platformId, query);
        return users.stream()
                .filter(u -> u.getDeletedAt() == null)
                .map(this::mapToDTO)
                .collect(Collectors.toList());
    }

    public List<UserDTO> getUsersByPlatform(String platformId) {
        List<User> users = userRepository.findByPlatformId(platformId);
        return users.stream()
                .filter(u -> u.getDeletedAt() == null)
                .map(this::mapToDTO)
                .collect(Collectors.toList());
    }

    public boolean validatePassword(String rawPassword, String encodedPassword) {
        return passwordEncoder.matches(rawPassword, encodedPassword);
    }

    public void changePassword(String userId, String newPassword) {
        User user = userRepository.findById(userId)
                .orElseThrow(() -> new RuntimeException("User not found: " + userId));
        user.setPasswordHash(passwordEncoder.encode(newPassword));
        user.setUpdatedAt(java.time.LocalDateTime.now());
        userRepository.save(user);
    }

    private UserDTO mapToDTO(User user) {
        return UserDTO.builder()
                .id(user.getId())
                .name(user.getName())
                .username(user.getUsername())
                .email(user.getEmail())
                .headline(user.getHeadline())
                .avatarUrl(user.getAvatarUrl())
                .coverPhotoUrl(user.getCoverPhotoUrl())
                .bio(user.getBio())
                .location(user.getLocation())
                .website(user.getWebsite())
                .whatsappNumber(user.getWhatsappNumber())
                .province(user.getProvince())
                .role(user.getRole())
                .platformId(user.getPlatformId())
                .interests(user.getInterests())
                .experiences(mapExperiences(user.getExperiences()))
                .education(mapEducation(user.getEducation()))
                .skills(user.getSkills())
                .certifications(mapCertifications(user.getCertifications()))
                .socialLinks(mapSocialLinks(user.getSocialLinks()))
                .createdAt(user.getCreatedAt())
                .updatedAt(user.getUpdatedAt())
                .build();
    }
    
    private List<UserDTO.ExperienceDTO> mapExperiences(List<User.Experience> experiences) {
        if (experiences == null) return null;
        return experiences.stream()
                .map(e -> UserDTO.ExperienceDTO.builder()
                        .id(e.getId())
                        .title(e.getTitle())
                        .company(e.getCompany())
                        .location(e.getLocation())
                        .description(e.getDescription())
                        .startDate(e.getStartDate())
                        .endDate(e.getEndDate())
                        .currentlyWorking(e.getCurrentlyWorking())
                        .build())
                .collect(Collectors.toList());
    }
    
    private List<UserDTO.EducationDTO> mapEducation(List<User.Education> education) {
        if (education == null) return null;
        return education.stream()
                .map(e -> UserDTO.EducationDTO.builder()
                        .id(e.getId())
                        .school(e.getSchool())
                        .degree(e.getDegree())
                        .fieldOfStudy(e.getFieldOfStudy())
                        .startDate(e.getStartDate())
                        .endDate(e.getEndDate())
                        .description(e.getDescription())
                        .build())
                .collect(Collectors.toList());
    }
    
    private List<UserDTO.CertificationDTO> mapCertifications(List<User.Certification> certifications) {
        if (certifications == null) return null;
        return certifications.stream()
                .map(c -> UserDTO.CertificationDTO.builder()
                        .id(c.getId())
                        .name(c.getName())
                        .issuer(c.getIssuer())
                        .issueDate(c.getIssueDate())
                        .expirationDate(c.getExpirationDate())
                        .credentialUrl(c.getCredentialUrl())
                        .build())
                .collect(Collectors.toList());
    }
    
    private UserDTO.SocialLinksDTO mapSocialLinks(User.SocialLinks socialLinks) {
        if (socialLinks == null) return null;
        return UserDTO.SocialLinksDTO.builder()
                .linkedin(socialLinks.getLinkedin())
                .twitter(socialLinks.getTwitter())
                .github(socialLinks.getGithub())
                .portfolio(socialLinks.getPortfolio())
                .build();
    }


    public UserDTO registerUserWithProfile(String email, String password, String name,
                                          String whatsappNumber, String province, String platformId) {
        // Check if user already exists
        java.util.Optional<User> existingEmail = userRepository.findByEmail(email);
        if (existingEmail.isPresent()) {
            log.info("User with email {} already exists in registerUserWithProfile, returning existing user", email);
            return mapToDTO(existingEmail.get());
        }

        // Generate username from email (before @)
        String username = email.split("@")[0];

        User user = User.builder()
                .username(username)
                .email(email)
                .passwordHash(passwordEncoder.encode(password))
                .name(name)
                .whatsappNumber(whatsappNumber)
                .province(province)
                .role("member")
                .platformId(platformId)
                .createdAt(LocalDateTime.now())
                .updatedAt(LocalDateTime.now())
                .build();

        User savedUser = userRepository.save(user);

        log.info("User registered with profile: {}", email);
        UserDTO dto = mapToDTO(savedUser);
        publishUserEvent("USER_CREATED", dto);
        return dto;
    }

    public List<UserDTO> getAllUsers() {
        log.info("Fetching all users");
        return userRepository.findAll().stream()
                .filter(user -> user.getDeletedAt() == null)
                .map(this::mapToDTO)
                .toList();
    }

}
