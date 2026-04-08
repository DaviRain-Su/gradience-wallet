module Gradience
  class Error < StandardError
    attr_reader :status_code, :body

    def initialize(message, status_code = nil, body = nil)
      super(message)
      @status_code = status_code
      @body = body
    end
  end
end
