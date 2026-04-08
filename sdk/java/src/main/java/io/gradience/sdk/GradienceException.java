package io.gradience.sdk;

public class GradienceException extends Exception {
    private final Integer statusCode;
    private final String body;

    public GradienceException(String message, Integer statusCode, String body) {
        super(message);
        this.statusCode = statusCode;
        this.body = body;
    }

    public Integer getStatusCode() {
        return statusCode;
    }

    public String getBody() {
        return body;
    }
}
