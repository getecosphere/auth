package com.rwid.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;
import org.springframework.data.annotation.Id;
import org.springframework.data.mongodb.core.mapping.Document;

import java.time.LocalDateTime;
import java.util.List;

@Data
@NoArgsConstructor
@AllArgsConstructor
@Builder
@Document(collection = "app_config")
public class AppConfig {

    @Id
    private String id;

    private String appName;

    private String logoUrl;

    private String avatarUrl;

    private String tagline;

    private String homepageType;

    private String javaneseHeroTitle1;

    private String javaneseHeroTitle2;

    private String javaneseHeroSubtitle;

    private List<String> javaneseHeroFeatures;

    private LocalDateTime createdAt;

    private LocalDateTime updatedAt;
}
