package com.rwid.service;

import com.rwid.dto.AppConfigResponse;
import com.rwid.model.AppConfig;
import com.rwid.repository.AppConfigRepository;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;

import java.time.LocalDateTime;
import java.util.List;
import java.util.Optional;

@Slf4j
@Service
public class AppConfigService {

    private final AppConfigRepository appConfigRepository;

    public AppConfigService(AppConfigRepository appConfigRepository) {
        this.appConfigRepository = appConfigRepository;
    }

    public Optional<AppConfig> getAppConfig() {
        List<AppConfig> configs = appConfigRepository.findAll();
        if (configs.isEmpty()) {
            log.warn("No app config found");
            return Optional.empty();
        }
        return Optional.of(configs.get(0));
    }

    public AppConfig updateAppConfig(AppConfigResponse request) {
        List<AppConfig> configs = appConfigRepository.findAll();
        AppConfig config;
        
        if (configs.isEmpty()) {
            config = AppConfig.builder()
                    .appName(request.getAppName())
                    .tagline(request.getTagline())
                    .logoUrl(request.getLogoUrl())
                    .avatarUrl(request.getAvatarUrl())
                    .homepageType(request.getHomepageType() != null ? request.getHomepageType() : "fancy")
                    .javaneseHeroTitle1(request.getJavaneseHeroTitle1())
                    .javaneseHeroTitle2(request.getJavaneseHeroTitle2())
                    .javaneseHeroSubtitle(request.getJavaneseHeroSubtitle())
                    .javaneseHeroFeatures(request.getJavaneseHeroFeatures())
                    .createdAt(LocalDateTime.now())
                    .updatedAt(LocalDateTime.now())
                    .build();
        } else {
            config = configs.get(0);
            if (request.getAppName() != null) {
                config.setAppName(request.getAppName());
            }
            if (request.getTagline() != null) {
                config.setTagline(request.getTagline());
            }
            // Only update logoUrl if it's provided in the request
            if (request.getLogoUrl() != null) {
                config.setLogoUrl(request.getLogoUrl());
            }
            // Only update avatarUrl if it's provided in the request
            if (request.getAvatarUrl() != null) {
                config.setAvatarUrl(request.getAvatarUrl());
            }
            if (request.getHomepageType() != null) {
                config.setHomepageType(request.getHomepageType());
            } else if (config.getHomepageType() == null) {
                config.setHomepageType("fancy");
            }
            if (request.getJavaneseHeroTitle1() != null) {
                config.setJavaneseHeroTitle1(request.getJavaneseHeroTitle1());
            }
            if (request.getJavaneseHeroTitle2() != null) {
                config.setJavaneseHeroTitle2(request.getJavaneseHeroTitle2());
            }
            if (request.getJavaneseHeroSubtitle() != null) {
                config.setJavaneseHeroSubtitle(request.getJavaneseHeroSubtitle());
            }
            if (request.getJavaneseHeroFeatures() != null) {
                config.setJavaneseHeroFeatures(request.getJavaneseHeroFeatures());
            }
            config.setUpdatedAt(LocalDateTime.now());
        }
        
        return appConfigRepository.save(config);
    }
}
