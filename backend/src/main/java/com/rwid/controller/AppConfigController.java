package com.rwid.controller;

import com.rwid.dto.AppConfigResponse;
import com.rwid.model.AppConfig;
import com.rwid.service.AppConfigService;
import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

@Slf4j
@RestController
@RequestMapping("/app-config")
public class AppConfigController {

    private final AppConfigService appConfigService;

    @Value("${app.api-base-url:http://localhost:8080/api}")
    private String apiBaseUrl;

    public AppConfigController(AppConfigService appConfigService) {
        this.appConfigService = appConfigService;
    }

    @GetMapping
    public ResponseEntity<AppConfigResponse> getAppConfig() {
        log.info("Fetching app config");
        var config = appConfigService.getAppConfig();
        
        AppConfigResponse response = AppConfigResponse.builder()
                .apiBaseUrl(apiBaseUrl)
                .build();
        
        if (config.isPresent()) {
            response.setAppName(config.get().getAppName());
            response.setTagline(config.get().getTagline());
            response.setHomepageType(config.get().getHomepageType() != null ? config.get().getHomepageType() : "fancy");
            response.setJavaneseHeroTitle1(config.get().getJavaneseHeroTitle1());
            response.setJavaneseHeroTitle2(config.get().getJavaneseHeroTitle2());
            response.setJavaneseHeroSubtitle(config.get().getJavaneseHeroSubtitle());
            response.setJavaneseHeroFeatures(config.get().getJavaneseHeroFeatures());
            String logoUrl = config.get().getLogoUrl();
            // If logoUrl is relative, make it absolute using apiBaseUrl
            if (logoUrl != null && logoUrl.startsWith("/api")) {
                logoUrl = apiBaseUrl.replaceAll("/api$", "") + logoUrl;
            }
            response.setLogoUrl(logoUrl);
            
            String avatarUrl = config.get().getAvatarUrl();
            // If avatarUrl is relative, make it absolute using apiBaseUrl
            if (avatarUrl != null && avatarUrl.startsWith("/api")) {
                avatarUrl = apiBaseUrl.replaceAll("/api$", "") + avatarUrl;
            }
            response.setAvatarUrl(avatarUrl);
        }
        
        return ResponseEntity.ok(response);
    }
 
    @PutMapping
    public ResponseEntity<AppConfigResponse> updateAppConfig(@RequestBody AppConfigResponse request) {
        log.info("Updating app config");
        AppConfig updated = appConfigService.updateAppConfig(request);
        
        AppConfigResponse response = AppConfigResponse.builder()
                .appName(updated.getAppName())
                .tagline(updated.getTagline())
                .homepageType(updated.getHomepageType() != null ? updated.getHomepageType() : "fancy")
                .javaneseHeroTitle1(updated.getJavaneseHeroTitle1())
                .javaneseHeroTitle2(updated.getJavaneseHeroTitle2())
                .javaneseHeroSubtitle(updated.getJavaneseHeroSubtitle())
                .javaneseHeroFeatures(updated.getJavaneseHeroFeatures())
                .apiBaseUrl(apiBaseUrl)
                .build();
        
        String logoUrl = updated.getLogoUrl();
        if (logoUrl != null && logoUrl.startsWith("/api")) {
            logoUrl = apiBaseUrl.replaceAll("/api$", "") + logoUrl;
        }
        response.setLogoUrl(logoUrl);
        
        String avatarUrl = updated.getAvatarUrl();
        if (avatarUrl != null && avatarUrl.startsWith("/api")) {
            avatarUrl = apiBaseUrl.replaceAll("/api$", "") + avatarUrl;
        }
        response.setAvatarUrl(avatarUrl);
        
        return ResponseEntity.ok(response);
    }
}