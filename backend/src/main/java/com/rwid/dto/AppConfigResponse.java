package com.rwid.dto;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;
import java.util.List;

/**
 * Response DTO for app configuration exposed to frontend
 * Contains public configuration needed by the frontend
 */
@Data
@NoArgsConstructor
@AllArgsConstructor
@Builder
public class AppConfigResponse {
    private String appName;
    private String logoUrl;
    private String avatarUrl;
    private String tagline;
    private String homepageType;
    private String javaneseHeroTitle1;
    private String javaneseHeroTitle2;
    private String javaneseHeroSubtitle;
    private List<String> javaneseHeroFeatures;
    private String apiBaseUrl;
}
